use crate::prelude::*;

use crate::audio::nodes::{Node, ProcessContext};
use crate::audio::node_graph::{NodeKey, NodeSlot, NodeConnectivityGraph};
use crate::audio::system::EvaluationContext;
use crate::audio::scratch_buffer::ScratchBuffer;
use crate::audio::scratch_buffer_cache::ScratchBufferCache;

use petgraph::graph::NodeIndex;
use rayon::prelude::*;


/// Optimised representation of a NodeGraph.
pub(in crate::audio) struct ExecutionGraph {
	workgroups: Vec<WorkGroup>,

	/// Flattened list of references to all input buffers for each node.
	/// Indexed by NodeWorkItem::input_buffers_range.
	/// Lifetime of ScratchBuffers managed by ScratchBufferCache, which should outlive ExecutionGraph.
	input_buffer_ptrs: Vec<*const ScratchBuffer>,

	/// The buffer which will be written to by the output node of the node graph.
	/// Lifetime managed by ScratchBufferCache, which should outlive ExecutionGraph.
	output_buffer: *const ScratchBuffer,
}

unsafe impl Send for ExecutionGraph {}

impl ExecutionGraph {
	pub fn empty() -> ExecutionGraph {
		ExecutionGraph {
			workgroups: Vec::new(),
			input_buffer_ptrs: Vec::new(),
			output_buffer: std::ptr::null(),
		}
	}


	#[instrument(skip_all, name = "audio::ExecutionGraph::validate")]
	pub fn validate(&self) {
		use std::collections::HashSet;

		let mut seen: HashSet<*const ScratchBuffer> = HashSet::new();

		for workgroup in self.workgroups.iter() {
			seen.clear();

			for work_item in workgroup.work_items.iter() {
				assert!(seen.insert(work_item.output_buffer));

				let inputs = &self.input_buffer_ptrs[work_item.input_buffers_range.clone()];
				for &input_buffer in inputs {
					assert!(seen.insert(input_buffer));
				}
			}
		}
	}


	#[instrument(skip_all, name = "audio::ExecutionGraph::from_graph")]
	pub fn from_graph(graph: &NodeConnectivityGraph, nodes: &mut slotmap::SlotMap<NodeKey, NodeSlot>, eval_ctx: &EvaluationContext<'_>,
		output_node_index: NodeIndex, buffer_cache: &mut ScratchBufferCache)
		-> ExecutionGraph
	{
		use petgraph::visit::NodeIndexable;
		use petgraph::algo::{has_path_connecting, DfsSpace};
		

		#[derive(Debug)]
		struct NodeInfo {
			workgroup_idx: usize,
			buffer_idx: Option<usize>,
		}

		let mut node_info: Vec<Option<NodeInfo>> = std::iter::repeat_with(|| None).take(graph.node_bound()).collect();


		let mut to_visit: Vec<NodeIndex> = graph.externals(petgraph::Incoming).collect();
		let mut new_nodes = Vec::new();

		let mut intermediate_workgroups = Vec::new();

		#[derive(Debug)]
		struct IntermediateWorkItem {
			node_idx: NodeIndex,
			node_ptr: *mut (dyn Node + 'static),
		}

		#[derive(Debug)]
		struct IntermediateWorkGroup {
			work_items: Vec<IntermediateWorkItem>,
		}

		let mut dfs = DfsSpace::new(&graph);

		// Collect nodes into workgroups
		while !to_visit.is_empty() {
			let mut workgroup = IntermediateWorkGroup {
				work_items: Vec::new(),
			};

			let workgroup_idx = intermediate_workgroups.len();

			for node_idx in to_visit.drain(..) {
				// If node has already been visited and assigned to a workgroup, we don't need to create any new work items.
				if node_info[node_idx.index()].is_some() {
					continue
				}

				// Reject nodes that have unvisited inputs, or that have been visited in the current workgroup.
				let all_inputs_visited = graph.neighbors_directed(node_idx, petgraph::Incoming)
					.all(|idx| node_info[idx.index()].as_ref().map(|n| n.workgroup_idx < workgroup_idx) == Some(true));

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
					workgroup_idx: intermediate_workgroups.len(),
					buffer_idx: None,
				});

				let node_key = graph[node_idx];
				let node = &mut nodes[node_key].node;

				workgroup.work_items.push(IntermediateWorkItem{
					node_idx,
					node_ptr: unsafe {
						// SAFETY: this pointer is never moved through.
						node.as_mut().get_unchecked_mut()
					}
				});

				// Tentatively add outgoing neighbours.
				for neighbor_index in graph.neighbors(node_idx) {
					new_nodes.push(neighbor_index);
				}
			}

			std::mem::swap(&mut to_visit, &mut new_nodes);

			intermediate_workgroups.push(workgroup);
		}


		// Allocate buffers for each node
		#[derive(Debug)]
		struct BufferRequest {
			alive_until_workgroup: usize,
			stereo: bool,
		}

		let mut buffers: Vec<BufferRequest> = Vec::new();
		let mut free_mono_buffer_indices = Vec::new();
		let mut free_stereo_buffer_indices = Vec::new();

		for (workgroup_idx, workgroup) in intermediate_workgroups.iter_mut().enumerate() {
			for node in workgroup.work_items.iter_mut() {
				let latest_output_use = graph.neighbors(node.node_idx)
					.map(|neighbor_idx| node_info[neighbor_idx.index()].as_ref().unwrap().workgroup_idx)
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
					buffers[buffer_idx].alive_until_workgroup = latest_output_use;
				} else {
					node_info.buffer_idx = Some(buffers.len());
					buffers.push(BufferRequest {
						alive_until_workgroup: latest_output_use,
						stereo,
					});
				}
			}

			for (index, buffer) in buffers.iter().enumerate() {
				if buffer.alive_until_workgroup == workgroup_idx {
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

		let mut workgroups = Vec::new();
		let mut input_buffer_ptrs = Vec::new();

		for intermediate_workgroup in intermediate_workgroups {
			if intermediate_workgroup.work_items.is_empty() {
				continue
			}

			let mut work_items = Vec::new();

			for intermediate_work_item in intermediate_workgroup.work_items {
				let buffer_ptrs_start = input_buffer_ptrs.len();
				for neighbor in graph.neighbors_directed(intermediate_work_item.node_idx, petgraph::Incoming) {
					let neighbor_info = node_info[neighbor.index()].as_ref().unwrap();
					let buffer_idx = neighbor_info.buffer_idx.unwrap();

					input_buffer_ptrs.push(intermediate_buffers[buffer_idx] as *const _);
				}
				let buffer_ptrs_end = input_buffer_ptrs.len();

				let buffer_idx = node_info[intermediate_work_item.node_idx.index()].as_ref().unwrap().buffer_idx.unwrap();

				work_items.push(NodeWorkItem {
					node: intermediate_work_item.node_ptr,
					output_buffer: intermediate_buffers[buffer_idx],
					input_buffers_range: buffer_ptrs_start..buffer_ptrs_end,
				})
			}

			workgroups.push(WorkGroup {
				work_items,
			});
		}

		let output_buffer_idx = node_info[output_node_index.index()].as_ref().unwrap().buffer_idx.unwrap();
		let output_buffer = intermediate_buffers[output_buffer_idx];

		ExecutionGraph {
			workgroups,
			input_buffer_ptrs,
			output_buffer,
		}
	}

	// Calling this is unsafe because it is only safe to call with some external, unenforceable guarantees.
	// Mainly, neither the node storage nor the buffer cache may be modified between the construction of this ExecutionGraph
	// and calls to ExecutionGraph::process. Calling process after modifying either the node storage or buffer cache without rebuilding
	// the ExecutionGraph would result in race conditions and so is UB.
	#[instrument(skip_all, name = "audio::ExecutionGraph::process")]
	pub(in crate::audio) unsafe fn process(&mut self, eval_ctx: &EvaluationContext<'_>) -> &[f32] {
		use std::sync::atomic::{Ordering, AtomicUsize};

		for workgroup in self.workgroups.iter() {
			#[inline]
			unsafe fn dereference_ptr_slice<'s>(slice: &'s [*const ScratchBuffer]) -> &'s [&'s ScratchBuffer] {
				unsafe {
					std::slice::from_raw_parts(
						slice.as_ptr() as *const _,
						slice.len()
					)
				}
			}

			fn process_work_item(work_item: &NodeWorkItem, input_buffer_ptrs: &'_ [*const ScratchBuffer], eval_ctx: &EvaluationContext<'_>) {
				// SAFETY: these are guaranteed to be disjoint for all work_items within a workgroup, which is ensured by `validate()`.
				let output_buffer: &mut ScratchBuffer = unsafe{ &mut *work_item.output_buffer };

				// SAFETY: this range is generated at the same time that input_buffer_ptrs is generated and so is guaranteed to be in range.
				// The pointers within this range are also guaranteed not to equal work_item.output_buffer - so aliasing may never occur.
				let input_range = work_item.input_buffers_range.clone();
				let inputs_raw = unsafe { input_buffer_ptrs.get_unchecked(input_range) };
				let input_buffers: &[&ScratchBuffer] = dereference_ptr_slice(inputs_raw);

				let process_ctx = ProcessContext {
					eval_ctx,
					inputs: input_buffers,
					output: output_buffer,
				};

				// SAFETY: there is guaranteed to only be one work item for each node in the graph, so this is guaranteed to be the only
				// mutable reference to this node at this point.
				unsafe {
					(*work_item.node).process(process_ctx);
				}
			}

			let num_work_items = workgroup.work_items.len();

			// Minor optimisation - we don't gain anything by parallelising a single task, so just execute it synchronously.
			// For some tasks we also may not gain anything for larger numbers either, but this is harder to reason about generally.
			if num_work_items == 1 {
				let work_item = unsafe {
					workgroup.work_items.get_unchecked(0)
				};

				process_work_item(work_item, &self.input_buffer_ptrs, eval_ctx);
				continue
			}
			
			// We need rayon to let us resolve the input buffers for each work_item.
			// `*const T` is !Sync, and so &[*const T] is !Send - which is required by for_each_with.
			// We can be sure that this is safe because the pointers are all into ScratchBufferCache which outlives ExecutionGraph,
			// _and_ because both input and output buffers are guaranteed to be disjoint within a workgroup.
			#[derive(Copy, Clone)]
			struct TrustMeThisIsSendForNow<'a>(&'a [*const ScratchBuffer]);
			unsafe impl Send for TrustMeThisIsSendForNow<'_> {}

			let input_buffer_ptrs = TrustMeThisIsSendForNow(&self.input_buffer_ptrs);

			let current_job_idx = AtomicUsize::new(0);
			let num_threads = rayon::current_num_threads()
				.min(num_work_items);

			// Distribute work among rayons worker threads, but allocate actual work items via `current_job_idx` so we bypass
			// any extra work rayon is doing. This *should* give close to the best case utilisation of worker threads within this workgroup.
			(0..num_threads).into_par_iter()
				.for_each_with(input_buffer_ptrs, |ptrs, idx| {
					let _span = tracing::debug_span!("process workitems", idx);
					let _span = _span.enter();

					loop {
						let work_item_idx = current_job_idx.fetch_add(1, Ordering::Relaxed);
						if work_item_idx >= num_work_items {
							break;
						}

						let work_item = unsafe {
							workgroup.work_items.get_unchecked(work_item_idx)
						};
						process_work_item(work_item, ptrs.0, eval_ctx);
					}
				});
		}

		// SAFETY: it is guaranteed that at this point there are no threads modifying any ScratchBuffers referenced by this graph.
		unsafe {
			&*self.output_buffer
		}
	}
}


/// Represents a node in the NodeGraph which contributes to the final audio output, along with references
/// to all buffers needed to process it.
#[derive(Debug)]
struct NodeWorkItem {
	/// The node this work item represents.
	/// Lifetime of Node managed by NodeGraph, which will rebuild ExecutionGraph on any destructive changes.
	node: *mut dyn Node,

	/// The buffer this node should write its output to upon processing.
	/// May be written by other work items, but guaranteed to unique within a WorkGroup.
	/// Lifetime of ScratchBuffer managed by ScratchBufferCache, which should outlive ExecutionGraph.
	output_buffer: *mut ScratchBuffer,

	/// Indexes ExecutionGraph::input_buffer_ptrs - all buffers which are connected as inputs to this node.
	input_buffers_range: std::ops::Range<usize>,
}

// Required by rayon IntoParallelIterator for `&[T]`.
// NOT ACTUALLY SAFE!
// NodeWorkItem is only Sync once its collected into a WorkGroup, and only while Node storage is locked.
// This is true during `process` but NodeWorkItem should not be used anywhere else.
unsafe impl Sync for NodeWorkItem {}



/// A collection of NodeWorkItems which can be safely processed in parallel, without risk of racing.
#[derive(Debug)]
struct WorkGroup {
	work_items: Vec<NodeWorkItem>,
}
