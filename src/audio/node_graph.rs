use crate::prelude::*;
use crate::audio::{nodes::*};
use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::intermediate_buffer_cache::IntermediateBufferCache;
use crate::audio::system::EvaluationContext;
use crate::audio::MAX_NODE_INPUTS;

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

	/// Should this node be removed if it has no inputs
	ephemeral: bool,
}


pub struct NodeGraph {
	connectivity: StableGraph<NodeKey, (), petgraph::Directed>,
	nodes: slotmap::SlotMap<NodeKey, NodeSlot>,

	buffer_cache: IntermediateBufferCache,
	output_node_key: NodeKey,
	output_node_index: NodeIndex,

	/// A processed version of `connectivity` with redunant nodes removed
	pruned_connectivity: StableGraph<NodeKey, (), petgraph::Directed>,

	/// Node indices of `pruned_connectivity` sorted in evaluation order.
	ordered_node_cache: Vec<NodeIndex>,
	topology_dirty: bool,
}


// Public API.
impl NodeGraph {
	pub fn new() -> NodeGraph {
		let mut connectivity = StableGraph::new();
		let mut nodes: slotmap::SlotMap<_, NodeSlot> = slotmap::SlotMap::with_key();

		let output_node = MixerNode::new_stereo(1.0);
		let output_node_key = nodes.insert(NodeSlot {
			node: Box::new(output_node),
			ephemeral: false,
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

	pub fn add_node(&mut self, node: impl Node, ephemeral: bool) -> NodeId {
		let node_key = self.nodes.insert(NodeSlot {
			node: Box::new(node),
			ephemeral,
		});

		let node_index = self.connectivity.add_node(node_key);
		// I guess there's no reason to recalc ordered_node_cache until nodes are connected
		// self.topology_dirty = true;
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
	#[instrument(skip_all, name = "audio::NodeGraph::cleanup_finished_nodes")]
	pub(in crate::audio) fn cleanup_finished_nodes(&mut self, eval_ctx: &EvaluationContext<'_>) {
		use petgraph::visit::IntoNodeReferences;

		let mut finished_nodes = Vec::new();

		// not sure I like doing this automatically?
		for (node_index, &node_key) in self.connectivity.node_references() {
			let node_slot = &self.nodes[node_key];
			if node_slot.node.finished_playing(eval_ctx) {
				finished_nodes.push((node_index, node_key));
				continue
			}

			if !node_slot.ephemeral {
				continue
			}

			let num_incoming = self.connectivity.neighbors_directed(node_index, petgraph::Direction::Incoming).count();
			if num_incoming == 0 {
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

		// Remove nodes not connected to the output node
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

