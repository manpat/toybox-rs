use crate::prelude::*;
use std::collections::HashMap;
use std::mem::MaybeUninit;


const MAX_NODE_INPUTS: usize = 16;



pub struct AudioSystem {
	audio_queue: sdl2::audio::AudioQueue<f32>,
	buffer_size: usize,

	node_graph: NodeGraph,
}

impl AudioSystem {
	pub fn new(sdl_audio: sdl2::AudioSubsystem) -> Result<AudioSystem, Box<dyn Error>> {
		let desired_spec = sdl2::audio::AudioSpecDesired {
			freq: Some(44100),
			channels: Some(2),
			samples: Some(512),
		};

		let audio_queue = sdl_audio.open_queue(None, &desired_spec)?;
		audio_queue.resume();

		let spec = audio_queue.spec();
		assert!(spec.freq == 44100);
		assert!(spec.channels == 2);

		let buffer_size = spec.samples as usize * spec.channels as usize;

		Ok(AudioSystem {
			audio_queue,
			buffer_size,
			
			node_graph: NodeGraph::new(),
		})
	}


	pub fn update(&mut self) {
		let spec = self.audio_queue.spec();

		let threshold_size = 3.0 / 60.0 * spec.freq as f32 * spec.channels as f32;
		let threshold_size = threshold_size as u32 * std::mem::size_of::<f32>() as u32;

		let eval_ctx = EvaluationContext {
			sample_rate: spec.freq as f32,
		};

		println!("======");

		while self.audio_queue.size() < threshold_size {
			let output_buffer = self.node_graph.process(&eval_ctx);

			// Submit audio frame
			self.audio_queue.queue(&output_buffer);
		}
	}
}


slotmap::new_key_type! { struct NodeKey; }

type NodeIndex = petgraph::graph::NodeIndex;

struct NodeGraph {
	connectivity: petgraph::stable_graph::StableGraph<NodeKey, (), petgraph::Directed>,
	nodes: slotmap::SlotMap<NodeKey, Box<dyn Node>>,

	buffer_cache: BufferCache,
	output_node_index: NodeIndex,

	topology_dirty: bool,
	ordered_node_cache: Vec<NodeIndex>,
}

impl NodeGraph {
	fn new() -> NodeGraph {
		let mut connectivity = petgraph::stable_graph::StableGraph::new();
		let mut nodes: slotmap::SlotMap<_, Box<dyn Node>> = slotmap::SlotMap::with_key();

		// let output_node_index = connectivity.add_node(nodes.insert(Box::new(WidenNode)));
		let output_node_index = MixerNode::new_stereo(0.2, 0.0);
		let output_node_index = connectivity.add_node(nodes.insert(Box::new(output_node_index)));

		// TODO(pat.m): ensure there's only one output node

		let mut graph = NodeGraph {
			connectivity,
			nodes,
			buffer_cache: BufferCache::new(),
			output_node_index,

			ordered_node_cache: Vec::new(),
			topology_dirty: false,
		};


		let global_mixer_node = graph.add_node(MixerNode::new_stereo(1.0, 0.0), None);

		let mixer_node = graph.add_node(MixerNode::new_stereo(2.0, 0.5), global_mixer_node);
		for freq in [55.0, 330.0] {
			graph.add_node(OscillatorNode::new(freq), mixer_node);
		}

		let mixer_node = graph.add_node(MixerNode::new_stereo(1.0, -0.5), global_mixer_node);
		for freq in [220.0, 110.0, 550.0] {
			graph.add_node(OscillatorNode::new(freq), mixer_node);
		}

		graph.topology_dirty = true;

		graph
	}

	fn add_node(&mut self, node: impl Node + 'static, send_node_index: impl Into<Option<NodeIndex>>) -> NodeIndex {
		let node_key = self.nodes.insert(Box::new(node));
		let node_index = self.connectivity.add_node(node_key);
		let send_node_index = send_node_index.into().unwrap_or(self.output_node_index);
		self.connectivity.add_edge(node_index, send_node_index, ());
		node_index
	}

	fn process(&mut self, eval_ctx: &EvaluationContext) -> &[f32] {
		use petgraph::Direction;

		// Make sure there are no inuse buffers remaining from previous frames - this will ensure
		// buffers for outgoing external nodes are correctly collected.
		self.buffer_cache.mark_all_unused();

		println!("{} buffers, {} nodes", self.buffer_cache.unused_buffers.len(), self.nodes.len());

		// Recalculate node evaluation order if the topology of the connectivity graph has changed
		if self.topology_dirty {
			self.ordered_node_cache = petgraph::algo::toposort(&self.connectivity, None)
				.expect("Connectivity graph is not a DAG");

			// TODO(pat.m): trim outgoing externals not output_node_index

			self.topology_dirty = false;
		}


		for &node_index in self.ordered_node_cache.iter() {
			let node_key = self.connectivity[node_index];
			let node = &mut self.nodes[node_key];

			// Fetch a buffer large enough for the nodes output
			let mut output_buffer = self.buffer_cache.new_buffer(node.has_stereo_output());

			// Collect all inputs for this node
			let incoming_nodes = self.connectivity.neighbors_directed(node_index, Direction::Incoming)
				.map(|node_index| self.connectivity[node_index]);

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
			let num_outgoing_edges = if node_key != self.output_node_index {
				self.connectivity.edges_directed(node_index, Direction::Outgoing).count()
			} else {
				// if we're currently processing the output node then give it a fake 'use'
				// so that it doesn't get collected before we return it
				1
			};

			self.buffer_cache.post_buffer(node_key, output_buffer, num_outgoing_edges);
		}

		// Finally, request the buffer for the output node
		let output_node_key = self.connectivity[self.output_node_index];
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



struct BufferCache {
	unused_buffers: Vec<IntermediateBuffer>,
	inuse_buffers: HashMap<NodeKey, IntermediateBuffer>,
}

impl BufferCache {
	fn new() -> BufferCache {
		BufferCache {
			unused_buffers: Vec::new(),
			inuse_buffers: HashMap::new(),
		}
	}

	fn new_buffer(&mut self, stereo: bool) -> IntermediateBuffer {
		let buffer_size = 2048;
		let target_size = match stereo {
			false => buffer_size,
			true => 2*buffer_size,
		};

		let mut buffer = self.unused_buffers.pop()
			.unwrap_or_else(|| IntermediateBuffer { samples: Vec::new(), uses: 0, stereo });

		buffer.samples.resize(target_size, 0.0);
		buffer.stereo = stereo;

		buffer
	}

	fn get_buffer(&self, associated_node: NodeKey) -> Option<&IntermediateBuffer> {
		self.inuse_buffers.get(&associated_node)
	}

	fn post_buffer(&mut self, associated_node: NodeKey, mut buffer: IntermediateBuffer, uses: usize) {
		buffer.uses = uses;

		if let Some(prev_buffer) = self.inuse_buffers.insert(associated_node, buffer) {
			self.unused_buffers.push(prev_buffer);
		}
	}

	fn mark_used(&mut self, associated_node: NodeKey) {
		if let Some(buffer) = self.inuse_buffers.get_mut(&associated_node) {
			buffer.uses = buffer.uses.saturating_sub(1);
			if buffer.uses == 0 {
				self.unused_buffers.push(self.inuse_buffers.remove(&associated_node).unwrap());
			}
		}
	}

	fn mark_all_unused(&mut self) {
		for (_, buffer) in self.inuse_buffers.drain() {
			self.unused_buffers.push(buffer);
		}
	}
}


struct IntermediateBuffer {
	samples: Vec<f32>,

	// set to the number of outgoing edges of a node, decremented for every use
	// reclaimed once it reaches zero
	uses: usize,
	stereo: bool,
}

impl std::ops::Deref for IntermediateBuffer {
	type Target = [f32];
	fn deref(&self) -> &[f32] { &self.samples }
}

impl std::ops::DerefMut for IntermediateBuffer {
	fn deref_mut(&mut self) -> &mut [f32] { &mut self.samples }
}


struct EvaluationContext {
	sample_rate: f32,
}



trait Node {
	fn has_stereo_output(&self) -> bool;
	fn process(&mut self, eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer);
}




struct MixerNode {
	// parameter
	gain: f32,
	pan: f32, // [-1, 1]
	stereo: bool,
}


impl MixerNode {
	fn new(gain: f32) -> MixerNode {
		MixerNode { gain, stereo: false, pan: 0.0 }
	}

	fn new_stereo(gain: f32, pan: f32) -> MixerNode {
		MixerNode { gain, stereo: true, pan: pan.clamp(-1.0, 1.0) }
	}
}

impl Node for MixerNode {
	fn has_stereo_output(&self) -> bool { self.stereo }

	fn process(&mut self, _eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
		assert!(output.stereo == self.stereo);

		output.fill(0.0);

		if self.stereo {
			let r_pan_factor = self.pan / 2.0 + 0.5;
			let l_pan_factor = 1.0 - r_pan_factor;

			for input in inputs {
				if input.stereo {
					for ([out_l, out_r], &[in_l, in_r]) in output.array_chunks_mut::<2>().zip(input.array_chunks::<2>()) {
						// TODO(pat.m): how pan??????
						// Some kinda cursed matrix thing?
						*out_l += in_l * self.gain;
						*out_r += in_r * self.gain;
					}
				} else {
					for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
						*out_l += in_sample * self.gain * l_pan_factor;
						*out_r += in_sample * self.gain * r_pan_factor;
					}
				}
			}

		} else {
			for input in inputs {
				for (out_sample, &in_sample) in output.iter_mut().zip(input.iter()) {
					*out_sample += in_sample * self.gain;
				}
			}
		}
	}
}



struct OscillatorNode {
	// parameter
	freq: f32,

	// state
	phase: f32,
}


impl OscillatorNode {
	fn new(freq: f32) -> OscillatorNode {
		OscillatorNode {
			freq,
			phase: 0.0,
		}
	}
}

impl Node for OscillatorNode {
	fn has_stereo_output(&self) -> bool { false }

	fn process(&mut self, eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
		assert!(inputs.is_empty());

		let frame_period = TAU * self.freq / eval_ctx.sample_rate;

		for out_sample in output.iter_mut() {
			*out_sample = self.phase.sin();
			self.phase += frame_period;
		}

		self.phase %= TAU;
	}
}



struct WidenNode;

impl WidenNode {
	fn new() -> WidenNode { WidenNode }
}

impl Node for WidenNode {
	fn has_stereo_output(&self) -> bool { true }

	fn process(&mut self, _eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
		assert!(inputs.len() == 1);
		assert!(output.stereo);

		let input = &inputs[0];
		assert!(!input.stereo);

		for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
			*out_l = in_sample;
			*out_r = in_sample;
		}
	}
}