use crate::prelude::*;
use crate::audio::{nodes::*};
use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::intermediate_buffer_cache::IntermediateBufferCache;
use crate::audio::system::EvaluationContext;
use crate::audio::MAX_NODE_INPUTS;

use crate::utility::ResourceScopeID;

use petgraph::stable_graph::StableGraph;
use petgraph::graph::NodeIndex;
use std::mem::MaybeUninit;


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

	buffer_cache: IntermediateBufferCache,
	output_node_key: NodeKey,
	output_node_index: NodeIndex,

	/// A processed version of `connectivity` with redunant nodes removed
	pruned_connectivity: NodeConnectivityGraph,

	/// Node indices of `pruned_connectivity` sorted in evaluation order.
	ordered_node_cache: Vec<NodeIndex>,
	topology_dirty: bool,
}


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
			buffer_cache: IntermediateBufferCache::new(128),
			output_node_key,
			output_node_index,

			ordered_node_cache: Vec::new(),
			pruned_connectivity: StableGraph::new(),
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
	pub(in crate::audio) fn update_topology(&mut self) {
		use petgraph::algo::{has_path_connecting, DfsSpace};

		// Recalculate node evaluation order if the topology of the connectivity graph has changed
		if !self.topology_dirty {
			return;
		}

		let mut dfs = DfsSpace::new(&self.connectivity);

		// Remove nodes not connected to the output node - ensures we don't process nodes that don't contribute
		// to the final sound. e.g., unconnected nodes and node islands that may still be being constructed.
		self.pruned_connectivity = self.connectivity.filter_map(
			|node_idx, &node_key| {
				if node_idx == self.output_node_index {
					return Some(node_key);
				}

				if !has_path_connecting(&self.connectivity, node_idx, self.output_node_index, Some(&mut dfs)) {
					None
				} else {
					Some(node_key)
				}
			},

			|_, &edge_key| Some(edge_key),
		);

		// Calculate final evaluation order
		self.ordered_node_cache = petgraph::algo::toposort(&self.pruned_connectivity, None)
			.expect("Connectivity graph is not a DAG");

		for &node_index in self.ordered_node_cache.iter() {
			let num_incoming = self.pruned_connectivity.neighbors_directed(node_index, petgraph::Direction::Incoming).count();
			if num_incoming > MAX_NODE_INPUTS {
				println!("Node(#{node_index:?}) has too many inputs! Node graph will no longer behave correctly!");
			}
		}

		self.topology_dirty = false;
	}

	#[instrument(skip_all, name = "audio::NodeGraph::process")]
	pub(in crate::audio) fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		assert!(!self.topology_dirty);

		use petgraph::Direction;

		// Make sure there are no inuse buffers remaining from previous frames - this will ensure
		// buffers for outgoing external nodes are correctly collected.
		self.buffer_cache.mark_all_unused();

		for &node_index in self.ordered_node_cache.iter() {
			let node_key = self.pruned_connectivity[node_index];
			let node_slot = &mut self.nodes[node_key];

			// Fetch a buffer large enough for the nodes output
			let mut output_buffer = self.buffer_cache.new_buffer(node_slot.node.has_stereo_output(eval_ctx));

			// Collect all inputs for this node
			let incoming_nodes = self.pruned_connectivity.neighbors_directed(node_index, Direction::Incoming)
				.map(|node_index| self.pruned_connectivity[node_index]);

			let mut storage = [MaybeUninit::<&IntermediateBuffer>::uninit(); MAX_NODE_INPUTS];
			let input_node_buffers = incoming_nodes.clone()
				.map(|node_key|
					self.buffer_cache.get_buffer(node_key)
						.expect("Failed to get evaluated buffer!"));

			let input_buffers = init_fixed_buffer_from_iterator(&mut storage, input_node_buffers);

			// Update node state and completely fill output_buffer
			let process_ctx = ProcessContext {
				eval_ctx,
				inputs: input_buffers,
				output: &mut output_buffer,
			};

			node_slot.node.process(process_ctx);

			// Mark all input buffers as being used once - potentially collecting them for reuse
			for node_key in incoming_nodes {
				self.buffer_cache.mark_used(node_key);
			}

			// Associate the output buffer to the current node and how many outgoing edges it has
			// so it can be collected for reuse once each of the outgoing neighbor nodes are evaluated
			let num_outgoing_edges = if node_index != self.output_node_index {
				self.pruned_connectivity.edges_directed(node_index, Direction::Outgoing).count()
			} else {
				// if we're currently processing the output node then give it a fake 'use'
				// so that it doesn't get collected before we return it
				1
			};

			self.buffer_cache.post_buffer(node_key, output_buffer, num_outgoing_edges);
		}

		// Finally, request the buffer for the output node
		let output_node_key = self.pruned_connectivity[self.output_node_index];
		self.buffer_cache.get_buffer(output_node_key)
			.expect("No output node!")
	}
}



fn init_fixed_buffer_from_iterator<'s, T, I, const N: usize>(storage: &'s mut [MaybeUninit<T>; N], iter: I) -> &'s [T]
	where T: Copy, I: Iterator<Item=T>
{
	let mut initialized_count = 0;
	for (target, source) in storage.iter_mut().zip(iter) {
		target.write(source);
		initialized_count += 1;
	}

	unsafe {
		let initialized_slice = &storage[..initialized_count];
		std::mem::transmute::<&[MaybeUninit<T>], &[T]>(initialized_slice)
	}
}









/// Nodes at same depth can operate independently.
/// Buffers can be reused once their assigned node has no incomplete outputs.



type BufferIdx = usize;

struct Job {
	node: *mut dyn Node,
	output_buffer: *mut IntermediateBuffer,
	input_buffers: std::ops::Range<BufferIdx>,
}

struct IndependentJobSet {
	jobs: Vec<Job>
}


struct ExecutionGraph {
	independent_jobs: Vec<IndependentJobSet>,
	buffer_ptrs: Vec<*const IntermediateBuffer>,
	output_buffer: *const IntermediateBuffer,
}

impl ExecutionGraph {
	fn from_graph(graph: &NodeConnectivityGraph, nodes: &mut slotmap::SlotMap<NodeKey, NodeSlot>, eval_ctx: &EvaluationContext<'_>,
		output_node_index: NodeIndex)
		-> ExecutionGraph
	{
		use std::collection::VecDeque;
		use petgraph::visit::Visitable;
		use petgraph::Direction;

		let mut visit_map = graph.visit_map();
		let mut to_visit: VecDeque<(NodeIndex, usize)> = VecDeque::new();

		// extend_with_initials
		to_visit.extend(graph.externals(Direction::Incoming).map(|idx| (idx, 0)));


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


		let mut independent_jobs = Vec::new();

		// file:///C:/Users/patrick/Development/playbox-rs/target/doc/src/petgraph/visit/traversal.rs.html#314-317
		
		while let Some((node_idx, depth)) = to_visit.pop_front() {
			if visit_map.is_visited(node_idx) {
				continue
			}

			visit_map.visit(node_idx);

			if independent_jobs.len() <= depth {
				independent_jobs.resize_with(depth + 1, || IndependentJobSet {
					jobs: Vec::new()
				});
			}

			let node = &mut nodes[node_key].node;
			let node_is_stereo = node.has_stereo_output(eval_ctx);

			let job_set = &mut independent_jobs[depth];

			job_set.jobs.push(Job {
				node: node.as_mut_ptr(),
				output_buffer,
				input_buffers,
			});

			for neighbor in graph.neighbors(node_idx) {
				to_visit.push_back((neighbor, depth + 1));
			}
		}
	}

	#[instrument(skip_all, name = "audio::NodeGraph::process")]
	pub(in crate::audio) fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		assert!(!self.topology_dirty);

		use petgraph::Direction;

		for job_set in self.independent_jobs.iter() {
			fn convert(slice: &[*const Buffer]) -> &[&Buffer] {
				let ptr = slice.as_ptr() as *const _;
				unsafe {
					std::slice::from_raw_parts(ptr, slice.len())
				}
			}

			// in parallel
			for job in job_set.jobs.iter() {
				let output_buffer: &mut Buffer = unsafe{ &mut *job.output_buffer };
				let input_buffers: &[&Buffer] = convert(&self.buffer_ptrs[job.input_buffers_range]);

				let process_ctx = ProcessContext {
					eval_ctx,
					inputs: input_buffers,
					output: output_buffer,
				};

				(*job.node).process(process_ctx)
			}
		}

		unsafe {
			*self.output_buffer
		}
	}
}