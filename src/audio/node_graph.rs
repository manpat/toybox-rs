use crate::prelude::*;
use crate::audio::{nodes::*};
use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::system::EvaluationContext;

use crate::utility::ResourceScopeID;

use petgraph::stable_graph::StableGraph;
use petgraph::graph::NodeIndex;
use rayon::prelude::*;


// TODO(pat.m): associate flags with nodes
// allow nodes to be marked as 'persistent' or 'ephemeral'
// if ephemeral, clean up if no ancestor nodes are playing audio - e.g., fx chains
// if persistent, don't clean up even if there are no inputs generating audio - e.g., for mixer nodes with saved references



slotmap::new_key_type! {
	pub(in crate::audio) struct NodeKey;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
	index: NodeIndex,
	key: NodeKey,
}


struct NodeSlot {
	node: Box<dyn Node>,

	/// Node won't be culled even if it has no incoming connections.
	/// Note: Source nodes are implicitly pinned
	pinned_scope: Option<ResourceScopeID>,
}


type NodeConnectivityGraph = StableGraph<NodeKey, (), petgraph::Directed>;

pub struct NodeGraph {
	connectivity: NodeConnectivityGraph,
	nodes: slotmap::SlotMap<NodeKey, NodeSlot>,

	output_node_key: NodeKey,
	output_node_index: NodeIndex,

	buffer_cache: BufferCache,
	execution_graph: ExecutionGraph,

	/// If this is true, execution_graph is no longer safe to use and must be rebuilt.
	topology_dirty: bool,
}

unsafe impl Send for NodeGraph {}


// Public API.
impl NodeGraph {
	pub fn new(global_resource_scope: ResourceScopeID) -> NodeGraph {
		let mut connectivity = StableGraph::new();
		let mut nodes: slotmap::SlotMap<_, NodeSlot> = slotmap::SlotMap::with_key();

		let output_node = MixerNode::new_stereo(1.0);
		let output_node_key = nodes.insert(NodeSlot {
			node: Box::new(output_node),
			pinned_scope: Some(global_resource_scope),
		});

		let output_node_index = connectivity.add_node(output_node_key);

		NodeGraph {
			connectivity,
			nodes,
			output_node_key,
			output_node_index,

			buffer_cache: BufferCache::new(512),
			execution_graph: ExecutionGraph::empty(),

			topology_dirty: true,
		}
	}

	pub fn buffer_size(&self) -> usize {
		self.buffer_cache.buffer_size()
	}

	pub fn output_node(&self) -> NodeId {
		NodeId {
			index: self.output_node_index,
			key: self.output_node_key,
		}
	}

	pub fn add_node(&mut self, node: impl Node, send_node_id: impl Into<Option<NodeId>>) -> NodeId {
		let node_key = self.nodes.insert(NodeSlot {
			node: Box::new(node),
			pinned_scope: None,
		});

		let node_index = self.connectivity.add_node(node_key);

		if let Some(send_node_id) = send_node_id.into() {
			self.connectivity.add_edge(node_index, send_node_id.index, ());

			// Only need to recalculate topology when nodes are connected.
			self.topology_dirty = true;
		}

		NodeId { index: node_index, key: node_key }
	}

	pub fn add_send(&mut self, node: NodeId, target: NodeId) {
		self.connectivity.add_edge(node.index, target.index, ());
		self.topology_dirty = true;
	}

	pub fn add_sends(&mut self, sends: impl IntoIterator<Item=(NodeId, NodeId)>) {
		let edges = sends.into_iter().map(|(id_a, id_b)| (id_a.index, id_b.index));
		self.connectivity.extend_with_edges(edges);
		self.topology_dirty = true;
	}

	pub fn add_send_chain(&mut self, chain: &[NodeId]) {
		let edges = chain.array_windows()
			.map(|&[id_a, id_b]| (id_a, id_b));

		self.add_sends(edges);
	}

	// TODO(pat.m): this api sucks to use directly
	pub fn pin_node_to_scope(&mut self, node: NodeId, scope: impl Into<Option<ResourceScopeID>>) {
		self.nodes[node.key].pinned_scope = scope.into();
	}

	pub fn remove_node(&mut self, node: NodeId) {
		assert!(node.index != self.output_node_index, "Trying to remove output node");
		if let Some(key) = self.connectivity.remove_node(node.index) {
			assert!(node.key == key);
			self.nodes.remove(key);
			self.topology_dirty = true;
		}
	}
}

// Private API.
impl NodeGraph {
	// TODO(pat.m): Do this in producer thread and instead require that all new node chains be added
	// and connected to output atomically.
	#[instrument(skip_all, name = "audio::NodeGraph::cleanup_finished_nodes")]
	pub(in crate::audio) fn cleanup_finished_nodes(&mut self, eval_ctx: &EvaluationContext<'_>,
		expired_resource_scopes: &[ResourceScopeID])
	{
		use petgraph::algo::{has_path_connecting, DfsSpace};
		use petgraph::visit::IntoNodeReferences;

		let mut finished_nodes = Vec::new();
		let mut dfs = DfsSpace::new(&self.connectivity);

		for (node_index, &node_key) in self.connectivity.node_references() {
			if node_index == self.output_node_index {
				continue;
			}

			// Remove nodes that are either 'finished' or no longer connected to anything
			// producing sound (in the case of effects).
			let node_slot = &self.nodes[node_key];
			let node_type = node_slot.node.node_type(eval_ctx);

			match node_type {
				NodeType::Source => if node_slot.node.finished_playing(eval_ctx) {
					finished_nodes.push((node_index, node_key));
					continue;
				}

				// TODO(pat.m): This behaviour may not be as appropriate for effects like delay lines, that might
				// continue producing sound after its inputs are removed for some time. Needs thinking about.
				NodeType::Effect => if node_slot.pinned_scope.is_none() {
					let num_incoming = self.connectivity.neighbors_directed(node_index, petgraph::Direction::Incoming).count();
					if num_incoming == 0 {
						finished_nodes.push((node_index, node_key));
						continue;
					}
				}
			}

			// Remove nodes not connected to output.
			if !has_path_connecting(&self.connectivity, node_index, self.output_node_index, Some(&mut dfs)) {
				finished_nodes.push((node_index, node_key));
				continue;
			}

			// Remove nodes pinned to expired resource scopes.
			if let Some(scope_id) = node_slot.pinned_scope
				&& let Ok(_) = expired_resource_scopes.binary_search(&scope_id)
			{
				finished_nodes.push((node_index, node_key));
			}
		}

		// TODO(pat.m): can this be done without the temp vector?
		for (index, key) in finished_nodes {
			self.remove_node(NodeId{index, key});
		}
	}

	#[instrument(skip_all, name = "audio::NodeGraph::update_topology")]
	pub(in crate::audio) fn update_topology(&mut self, eval_ctx: &EvaluationContext<'_>) {
		// Recalculate node evaluation order if the topology of the connectivity graph has changed
		if !self.topology_dirty {
			return;
		}

		self.execution_graph = ExecutionGraph::from_graph(&self.connectivity, &mut self.nodes, eval_ctx,
			self.output_node_index, &mut self.buffer_cache);

		self.execution_graph.validate();

		self.topology_dirty = false;
	}

	#[instrument(skip_all, name = "audio::NodeGraph::process")]
	pub(in crate::audio) fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		assert!(!self.topology_dirty);

		self.execution_graph.process(eval_ctx)
	}
}




type BufferIdx = usize;

#[derive(Debug)]
struct Job {
	node: *mut dyn Node,
	output_buffer: *mut IntermediateBuffer,
	input_buffers_range: std::ops::Range<BufferIdx>,
}

unsafe impl Send for Job {}
unsafe impl Sync for Job {}

#[derive(Debug)]
struct IndependentJobSet {
	jobs: Vec<Job>
}


struct ExecutionGraph {
	independent_jobs: Vec<IndependentJobSet>,
	buffer_ptrs: Vec<*const IntermediateBuffer>,
	output_buffer: *const IntermediateBuffer,
}

impl ExecutionGraph {
	fn empty() -> ExecutionGraph {
		ExecutionGraph {
			independent_jobs: Vec::new(),
			buffer_ptrs: Vec::new(),
			output_buffer: std::ptr::null(),
		}
	}


	#[instrument(skip_all, name = "audio::ExecutionGraph::validate")]
	fn validate(&self) {
		use std::collections::HashSet;

		let mut seen: HashSet<*const IntermediateBuffer> = HashSet::new();

		for jobset in self.independent_jobs.iter() {
			seen.clear();

			for job in jobset.jobs.iter() {
				assert!(seen.insert(job.output_buffer));

				let inputs = &self.buffer_ptrs[job.input_buffers_range.clone()];
				for &input_buffer in inputs {
					assert!(seen.insert(input_buffer));
				}
			}
		}
	}


	#[instrument(skip_all, name = "audio::ExecutionGraph::from_graph")]
	fn from_graph(graph: &NodeConnectivityGraph, nodes: &mut slotmap::SlotMap<NodeKey, NodeSlot>, eval_ctx: &EvaluationContext<'_>,
		output_node_index: NodeIndex, buffer_cache: &mut BufferCache)
		-> ExecutionGraph
	{
		use petgraph::Direction;
		use petgraph::visit::NodeIndexable;
		use petgraph::algo::{has_path_connecting, DfsSpace};


		// create separate allocators for stereo and mono buffers.
		// for each buffer store a list of ranges that it is in use for.
		//	determine this from the 'depth' of the node being allocated for, and the max depths of each of its outgoing neighbors
		//	use this to determine which buffers are safe to reuse.
		// Allocate all the buffers so pointers can safely be made to them, and stored in Jobs.
		// For each node:
		// - collect buffers pointers from incoming nodes and append into buffer_ptrs, save range
		// - store pointer to output buffer in job

		// To ensure:
		// - within a set, buffer pointers only appear ONCE as both inputs and outputs for jobs
		// - buffers aren't reused until all dependent nodes have been evaluated
		//	- this should be guaranteed with this collection into 'sets' - each set is effectively separated by a barrier


		// file:///C:/Users/patrick/Development/playbox-rs/target/doc/src/petgraph/visit/traversal.rs.html#314-317
		

		#[derive(Debug)]
		struct NodeInfo {
			wavefront_idx: usize,
			buffer_idx: Option<usize>,
		}

		let mut node_info: Vec<Option<NodeInfo>> = std::iter::repeat_with(|| None).take(graph.node_bound()).collect();


		let mut to_visit: Vec<NodeIndex> = graph.externals(Direction::Incoming).collect();
		let mut new_nodes = Vec::new();

		let mut wavefronts = Vec::new();

		#[derive(Debug)]
		struct WavefrontNode {
			node_idx: NodeIndex,
			node_ptr: *mut (dyn Node + 'static),
		}

		#[derive(Debug)]
		struct Wavefront {
			nodes: Vec<WavefrontNode>,
		}

		let mut dfs = DfsSpace::new(&graph);

		// Collect nodes into wavefronts
		while !to_visit.is_empty() {
			let mut wavefront = Wavefront {
				nodes: Vec::new(),
			};

			let wavefront_idx = wavefronts.len();

			for node_idx in to_visit.drain(..) {
				if node_info[node_idx.index()].is_some() {
					continue
				}

				// Reject nodes that have unvisited inputs, or that have been visited in the current wavefront.
				let all_inputs_visited = graph.neighbors_directed(node_idx, petgraph::Incoming)
					.all(|idx| node_info[idx.index()].as_ref().map(|n| n.wavefront_idx < wavefront_idx) == Some(true));

				if !all_inputs_visited {
					continue
				}

				// Reject nodes not connected to output
				if node_idx != output_node_index
					&& !has_path_connecting(&graph, node_idx, output_node_index, Some(&mut dfs))
				{
					continue
				}

				node_info[node_idx.index()] = Some(NodeInfo {
					wavefront_idx: wavefronts.len(),
					buffer_idx: None,
				});

				let node_key = graph[node_idx];
				let node = &mut nodes[node_key].node;

				wavefront.nodes.push(WavefrontNode{
					node_idx,
					node_ptr: &mut **node,
				});

				// Tentatively add outgoing neighbours
				for neighbor_index in graph.neighbors(node_idx) {
					new_nodes.push(neighbor_index);
				}
			}

			std::mem::swap(&mut to_visit, &mut new_nodes);

			wavefronts.push(wavefront);
		}


		// Allocate buffers for each node
		#[derive(Debug)]
		struct BufferRequest {
			alive_until_wavefront: usize,
			stereo: bool,
		}

		let mut buffers: Vec<BufferRequest> = Vec::new();
		let mut free_mono_buffer_indices = Vec::new();
		let mut free_stereo_buffer_indices = Vec::new();

		for (wavefront_idx, wavefront) in wavefronts.iter_mut().enumerate() {
			for node in wavefront.nodes.iter_mut() {
				let latest_output_use = graph.neighbors(node.node_idx)
					.map(|neighbor_idx| node_info[neighbor_idx.index()].as_ref().unwrap().wavefront_idx)
					.max()
					.unwrap_or(usize::MAX);


				let stereo = unsafe { (*node.node_ptr).has_stereo_output(eval_ctx) };

				let free_buffers = match stereo {
					false => &mut free_mono_buffer_indices,
					true => &mut free_stereo_buffer_indices,
				};

				let node_info = node_info[node.node_idx.index()].as_mut().unwrap();

				if let Some(buffer_idx) = free_buffers.pop() {
					node_info.buffer_idx = Some(buffer_idx);
					buffers[buffer_idx].alive_until_wavefront = latest_output_use;
				} else {
					node_info.buffer_idx = Some(buffers.len());
					buffers.push(BufferRequest {
						alive_until_wavefront: latest_output_use,
						stereo,
					});
				}
			}

			for (index, buffer) in buffers.iter().enumerate() {
				if buffer.alive_until_wavefront == wavefront_idx {
					let free_buffers = match buffer.stereo {
						false => &mut free_mono_buffer_indices,
						true => &mut free_stereo_buffer_indices,
					};

					free_buffers.push(index);
				}
			}
		}


		let stereo_buffer_count = buffers.iter().filter(|b| b.stereo).count();
		let mono_buffer_count = buffers.len() - stereo_buffer_count;

		buffer_cache.reset(mono_buffer_count, stereo_buffer_count);

		let intermediate_buffers = buffers.into_iter()
			.map(|request| buffer_cache.new_buffer(request.stereo))
			.collect::<Vec<_>>();

		let mut job_sets = Vec::new();
		let mut buffer_ptrs = Vec::new();

		for wavefront in wavefronts {
			if wavefront.nodes.is_empty() {
				continue
			}

			let mut jobs = Vec::new();

			for node in wavefront.nodes {
				let buffer_ptrs_start = buffer_ptrs.len();
				for neighbor in graph.neighbors_directed(node.node_idx, petgraph::Incoming) {
					let neighbor_info = node_info[neighbor.index()].as_ref().unwrap();
					let buffer_idx = neighbor_info.buffer_idx.unwrap();

					buffer_ptrs.push(intermediate_buffers[buffer_idx] as *const _);
				}
				let buffer_ptrs_end = buffer_ptrs.len();

				let buffer_idx = node_info[node.node_idx.index()].as_ref().unwrap().buffer_idx.unwrap();

				jobs.push(Job {
					node: node.node_ptr,
					output_buffer: intermediate_buffers[buffer_idx],
					input_buffers_range: buffer_ptrs_start..buffer_ptrs_end,
				})
			}

			job_sets.push(IndependentJobSet {
				jobs,
			});
		}

		let output_buffer_idx = node_info[output_node_index.index()].as_ref().unwrap().buffer_idx.unwrap();
		let output_buffer = intermediate_buffers[output_buffer_idx];

		ExecutionGraph {
			independent_jobs: job_sets,
			buffer_ptrs,
			output_buffer,
		}
	}

	#[instrument(skip_all, name = "audio::NodeGraph::process")]
	pub(in crate::audio) fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		use petgraph::Direction;

		for job_set in self.independent_jobs.iter() {
			fn convert(slice: &[*const IntermediateBuffer]) -> &[&IntermediateBuffer] {
				let ptr = slice.as_ptr() as *const _;
				unsafe {
					std::slice::from_raw_parts(ptr, slice.len())
				}
			}
			
			let jobs: &[Job] = &job_set.jobs;

			#[derive(Copy, Clone)]
			struct BufferPtrs<'a>(&'a [*const IntermediateBuffer]);

			unsafe impl Send for BufferPtrs<'_> {}
			unsafe impl Sync for BufferPtrs<'_> {}

			let buffer_ptrs = BufferPtrs(&self.buffer_ptrs);

			jobs.par_iter().for_each(move |job| {
				let buffer_ptrs = buffer_ptrs;
				let output_buffer: &mut IntermediateBuffer = unsafe{ &mut *job.output_buffer };
				let input_buffers: &[&IntermediateBuffer] = convert(&buffer_ptrs.0[job.input_buffers_range.clone()]);

				let process_ctx = ProcessContext {
					eval_ctx,
					inputs: input_buffers,
					output: output_buffer,
				};

				unsafe {
					(*job.node).process(process_ctx);
				}
			});
		}

		unsafe {
			&*self.output_buffer
		}
	}
}



struct BufferCache {
	mono_buffers: Vec<IntermediateBuffer>,
	mono_allocated_index: usize,

	stereo_buffers: Vec<IntermediateBuffer>,
	stereo_allocated_index: usize,
	buffer_size: usize,
}

impl BufferCache {
	fn new(buffer_size: usize) -> BufferCache {
		BufferCache {
			mono_buffers: Vec::new(),
			mono_allocated_index: 0,
			stereo_buffers: Vec::new(),
			stereo_allocated_index: 0,
			buffer_size,
		}
	}

	fn buffer_size(&self) -> usize {
		self.buffer_size
	}

	fn reset(&mut self, mono_buffer_count: usize, stereo_buffer_count: usize) {
		tracing::info!("BufferCache::reset! {} {}", mono_buffer_count, stereo_buffer_count);

		if self.mono_buffers.len() < mono_buffer_count {
			self.mono_buffers.resize_with(mono_buffer_count, || IntermediateBuffer::new(self.buffer_size, false));
		}

		if self.stereo_buffers.len() < stereo_buffer_count {
			self.stereo_buffers.resize_with(stereo_buffer_count, || IntermediateBuffer::new(self.buffer_size, true));
		}

		self.mono_allocated_index = 0;
		self.stereo_allocated_index = 0;
	}

	fn new_buffer(&mut self, stereo: bool) -> *mut IntermediateBuffer {
		let (buffers, allocated_index) = match stereo {
			false => (&mut self.mono_buffers, &mut self.mono_allocated_index),
			true => (&mut self.stereo_buffers, &mut self.stereo_allocated_index),
		};

		assert!(*allocated_index < buffers.len());
		let buffer = &mut buffers[*allocated_index];
		*allocated_index += 1;
		buffer
	}
}