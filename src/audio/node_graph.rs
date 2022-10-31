use crate::prelude::*;
use crate::audio::{nodes::*};
use crate::audio::system::EvaluationContext;
use crate::audio::execution_graph::ExecutionGraph;
use crate::audio::scratch_buffer_cache::ScratchBufferCache;

use crate::utility::ResourceScopeID;

use petgraph::stable_graph::StableGraph;
use petgraph::graph::NodeIndex;

use std::pin::Pin;

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


// This being exported is kinda gnarly but is the easiest way to expose node storage to ExecutionGraph.
pub(in crate::audio) struct NodeSlot {
	pub node: Pin<Box<dyn Node>>,

	/// Node won't be culled even if it has no incoming connections.
	/// Note: Source nodes are implicitly pinned
	pinned_scope: Option<ResourceScopeID>,
}

// This being exported is kinda gnarly but is the easiest way to expose this to ExecutionGraph.
pub(in crate::audio) type NodeConnectivityGraph = StableGraph<NodeKey, (), petgraph::Directed>;


pub struct NodeGraph {
	// Describes the connectivity betwen nodes in the graph.
	// Processed into a more optimal form in `update_topology`.
	connectivity: NodeConnectivityGraph,

	// Storage for all nodes in the graph.
	// Must not be modified between when `execution_graph` is initialised and when `ExecutionGraph::process` is called,
	// as mutable references into it are held by the execution graph.
	nodes: slotmap::SlotMap<NodeKey, NodeSlot>,

	// Stores all the ScratchBuffers that `execution_graph` will use during processing.
	// Must not be modified between when `execution_graph` is initialised and when `ExecutionGraph::process` is called.
	buffer_cache: ScratchBufferCache,

	// An optimised representation of the graph used only for evaluation.
	// Holds references to other members of NodeGraph and so must be recreated whenever the topology changes or nodes are removed.
	execution_graph: ExecutionGraph,

	output_node_key: NodeKey,
	output_node_index: NodeIndex,

	/// If this is true, execution_graph is no longer safe to use and must be rebuilt.
	topology_dirty: bool,
}



// Public API.
impl NodeGraph {
	pub fn new(global_resource_scope: ResourceScopeID) -> NodeGraph {
		let mut connectivity = StableGraph::new();
		let mut nodes: slotmap::SlotMap<_, NodeSlot> = slotmap::SlotMap::with_key();

		let output_node = MixerNode::new_stereo(1.0);
		let output_node_key = nodes.insert(NodeSlot {
			node: Box::pin(output_node),
			pinned_scope: Some(global_resource_scope),
		});

		let output_node_index = connectivity.add_node(output_node_key);

		NodeGraph {
			connectivity,
			nodes,
			output_node_key,
			output_node_index,

			buffer_cache: ScratchBufferCache::new(512),
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
			node: Box::pin(node),
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

	#[instrument(skip_all, name = "audio::NodeGraph::process", fields(samples=self.buffer_cache.buffer_size()))]
	pub(in crate::audio) fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		assert!(!self.topology_dirty);

		// SAFETY: The above assert and the unique reference to self guarantee that the below is safe.
		unsafe {
			self.execution_graph.process(eval_ctx)
		}
	}
}


