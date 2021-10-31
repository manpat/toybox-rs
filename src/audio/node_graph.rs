use crate::prelude::*;
use crate::audio::{nodes::*};
use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::system::EvaluationContext;
use crate::audio::buffer_cache::BufferCache;
use crate::audio::MAX_NODE_INPUTS;

use petgraph::stable_graph::StableGraph;
use petgraph::graph::NodeIndex;
use std::mem::MaybeUninit;


slotmap::new_key_type! { pub(in crate::audio) struct NodeKey; }

pub(in crate::audio) struct NodeGraph {
	connectivity: StableGraph<NodeKey, (), petgraph::Directed>,
	nodes: slotmap::SlotMap<NodeKey, Box<dyn Node>>,

	buffer_cache: BufferCache,
	output_node_index: NodeIndex,

	// A processed version of `connectivity` with redunant nodes removed
	pruned_connectivity: StableGraph<NodeKey, (), petgraph::Directed>,

	// Node indices of `pruned_connectivity` sorted in evaluation order.
	ordered_node_cache: Vec<NodeIndex>,
	topology_dirty: bool,
}

impl NodeGraph {
	pub fn new() -> NodeGraph {
		let mut connectivity = StableGraph::new();
		let mut nodes: slotmap::SlotMap<_, Box<dyn Node>> = slotmap::SlotMap::with_key();

		let output_node_index = MixerNode::new_stereo(1.0);
		let output_node_index = connectivity.add_node(nodes.insert(Box::new(output_node_index)));

		NodeGraph {
			connectivity,
			nodes,
			buffer_cache: BufferCache::new(1024),
			output_node_index,

			ordered_node_cache: Vec::new(),
			pruned_connectivity: StableGraph::new(),
			topology_dirty: false,
		}
	}

	pub fn output_node(&self) -> NodeIndex {
		self.output_node_index
	}

	pub fn add_node(&mut self, node: impl Node) -> NodeIndex {
		let node_key = self.nodes.insert(Box::new(node));
		let node_index = self.connectivity.add_node(node_key);
		// I guess there's no reason to recalc ordered_node_cache until nodes are connected
		// self.topology_dirty = true;
		node_index
	}

	pub fn add_send(&mut self, node: NodeIndex, target: NodeIndex) {
		self.connectivity.add_edge(node, target, ());
		self.topology_dirty = true;
	}

	pub fn remove_node(&mut self, node: NodeIndex) {
		assert!(node != self.output_node_index, "Trying to remove output node");
		if let Some(key) = self.connectivity.remove_node(node) {
			self.nodes.remove(key);
			self.topology_dirty = true;
		}
	}

	pub fn update(&mut self) {
		use petgraph::algo::{has_path_connecting, DfsSpace};

		// Recalculate node evaluation order if the topology of the connectivity graph has changed
		if !self.topology_dirty {
			return;
		}

		let mut dfs = DfsSpace::new(&self.connectivity);

		// Remove nodes not connected to the output node
		self.pruned_connectivity = self.connectivity.filter_map(
			|node_idx, &node_key| {
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

		self.topology_dirty = false;
	}

	pub fn process(&mut self, eval_ctx: &EvaluationContext) -> &[f32] {
		assert!(!self.topology_dirty);

		use petgraph::Direction;

		// Make sure there are no inuse buffers remaining from previous frames - this will ensure
		// buffers for outgoing external nodes are correctly collected.
		self.buffer_cache.mark_all_unused();

		println!("{} buffers, {} nodes", self.buffer_cache.total_buffer_count(), self.nodes.len());

		for &node_index in self.ordered_node_cache.iter() {
			let node_key = self.pruned_connectivity[node_index];
			let node = &mut self.nodes[node_key];

			// Fetch a buffer large enough for the nodes output
			let mut output_buffer = self.buffer_cache.new_buffer(node.has_stereo_output());

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
			node.process(&eval_ctx, input_buffers, &mut output_buffer);

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
	for (index, (target, source)) in storage.iter_mut().zip(iter).enumerate() {
		target.write(source);
		initialized_count = index+1;
	}

	unsafe {
		let initialised_slice = &storage[..initialized_count];
		std::mem::transmute::<&[MaybeUninit<T>], &[T]>(initialised_slice)
	}
}

