use crate::prelude::*;
use crate::audio::runtime::{system, system::EvaluationContext, scratch_buffer::ScratchBuffer};

mod mixer_node;
mod compressor_node;

pub use mixer_node::*;
pub use compressor_node::*;



pub enum NodeType {
	Source,
	Effect,
}

pub trait Node: 'static + Send + Sync {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool;
	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType;
	fn finished_playing(&self, _: &EvaluationContext<'_>) -> bool { false }
	fn process(&mut self, _: ProcessContext<'_>);
}


pub struct ProcessContext<'ctx> {
	pub eval_ctx: &'ctx EvaluationContext<'ctx>,
	pub inputs: &'ctx [&'ctx ScratchBuffer],
	pub output: &'ctx mut ScratchBuffer,
}







pub struct PannerNode {
	// parameter
	pan: f32, // [-1, 1]
}

impl PannerNode {
	pub fn new(pan: f32) -> PannerNode {
		PannerNode { pan: pan.clamp(-1.0, 1.0) }
	}
}

impl Node for PannerNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	fn process(&mut self, ProcessContext{inputs, output, ..}: ProcessContext<'_>) {
		assert!(output.stereo());

		let input = &inputs[0];
		assert!(!input.stereo());

		let r_pan = self.pan / 2.0 + 0.5;
		let l_pan = 1.0 - r_pan;

		for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
			*out_l = in_sample * l_pan;
			*out_r = in_sample * r_pan;
		}
	}
}




pub struct WidenNode;

impl WidenNode {
	pub fn new() -> WidenNode { WidenNode }
}

impl Node for WidenNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	fn process(&mut self, ProcessContext{inputs, output, ..}: ProcessContext<'_>) {
		assert!(inputs.len() == 1);
		assert!(output.stereo());

		let input = &inputs[0];
		assert!(!input.stereo());

		for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
			*out_l = in_sample;
			*out_r = in_sample;
		}
	}
}




pub struct SamplerNode {
	sound_id: system::SoundId,
	position: usize,
}

impl SamplerNode {
	pub fn new(sound_id: system::SoundId) -> SamplerNode {
		SamplerNode {
			sound_id,
			position: 0,
		}
	}
}


impl Node for SamplerNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Source }

	fn finished_playing(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		let buffer = eval_ctx.resources.get(self.sound_id);
		self.position >= buffer.len()
	}

	#[instrument(skip_all, name = "SamplerNode::process")]
	fn process(&mut self, ProcessContext{eval_ctx, inputs, output}: ProcessContext<'_>) {
		assert!(inputs.is_empty());
		assert!(!output.stereo());

		let buffer = eval_ctx.resources.get(self.sound_id);

		if self.position >= buffer.len() {
			output.fill(0.0);
			return;
		}

		let buffer_remaining = &buffer[self.position..];

		for (out_sample, in_sample) in output.iter_mut().zip(buffer_remaining) {
			*out_sample = *in_sample;
		}

		// Fill rest
		if buffer_remaining.len() < output.len() {
			output[buffer_remaining.len()..].fill(0.0);
		}

		self.position += output.len();
	}
}


