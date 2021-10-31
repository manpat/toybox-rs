use crate::prelude::*;
use crate::audio::nodes::Node;
use crate::audio::node_graph::NodeGraph;

use crate::audio::nodes::*;

pub struct EvaluationContext {
	pub sample_rate: f32,
}

use petgraph::graph::NodeIndex;

pub struct AudioSystem {
	audio_queue: sdl2::audio::AudioQueue<f32>,
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

		let mut system = AudioSystem {
			audio_queue,
			node_graph: NodeGraph::new(),
		};

		let global_mixer_node = system.add_node_with_send(MixerNode::new_stereo(0.2), system.output_node());

		let panner_node = system.add_node_with_send(PannerNode::new(1.0), global_mixer_node);
		let mixer_node = system.add_node_with_send(MixerNode::new(2.0), panner_node);
		for freq in [55.0, 330.0] {
			system.add_node_with_send(OscillatorNode::new(freq), mixer_node);
		}

		let panner_node = system.add_node_with_send(PannerNode::new(-1.0), global_mixer_node);
		let mixer_node = system.add_node_with_send(MixerNode::new(1.0), panner_node);
		for freq in [220.0, 110.0, 550.0] {
			system.add_node_with_send(OscillatorNode::new(freq), mixer_node);
		}

		Ok(system)
	}


	pub fn update(&mut self) {
		let spec = self.audio_queue.spec();

		let threshold_size = 1.0 / 60.0 * spec.freq as f32 * spec.channels as f32;
		let threshold_size = threshold_size as u32 * std::mem::size_of::<f32>() as u32;

		let eval_ctx = EvaluationContext {
			sample_rate: spec.freq as f32,
		};

		println!("======");

		self.node_graph.update();

		while self.audio_queue.size() < threshold_size {
			let output_buffer = self.node_graph.process(&eval_ctx);

			// Submit audio frame
			self.audio_queue.queue(&output_buffer);
		}
	}

	pub fn output_node(&self) -> NodeIndex {
		self.node_graph.output_node()
	}

	pub fn add_node(&mut self, node: impl Node) -> NodeIndex {
		self.node_graph.add_node(node)
	}

	pub fn add_send(&mut self, node: NodeIndex, target: NodeIndex) {
		self.node_graph.add_send(node, target)
	}

	pub fn add_node_with_send(&mut self, node: impl Node, send_node: NodeIndex) -> NodeIndex {
		let node_index = self.node_graph.add_node(node);
		self.node_graph.add_send(node_index, send_node);
		node_index
	}

	pub fn remove_node(&mut self, node: NodeIndex) {
		self.node_graph.remove_node(node)
	}
}