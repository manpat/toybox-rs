use crate::prelude::*;
use crate::audio::{system::EvaluationContext, intermediate_buffer::IntermediateBuffer};


pub trait Node: 'static {
	fn has_stereo_output(&self) -> bool;
	fn process(&mut self, eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer);
}






pub struct MixerNode {
	// parameter
	gain: f32,
	stereo: bool,
}


impl MixerNode {
	pub fn new(gain: f32) -> MixerNode {
		MixerNode { gain, stereo: false }
	}

	pub fn new_stereo(gain: f32) -> MixerNode {
		MixerNode { gain, stereo: true }
	}
}

impl Node for MixerNode {
	fn has_stereo_output(&self) -> bool { self.stereo }

	fn process(&mut self, _eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
		assert!(output.stereo() == self.stereo);

		output.fill(0.0);

		if self.stereo {
			for input in inputs {
				if input.stereo() {
					for ([out_l, out_r], &[in_l, in_r]) in output.array_chunks_mut::<2>().zip(input.array_chunks::<2>()) {
						*out_l += in_l * self.gain;
						*out_r += in_r * self.gain;
					}
				} else {
					for ([out_l, out_r], &in_sample) in output.array_chunks_mut::<2>().zip(input.iter()) {
						*out_l += in_sample * self.gain;
						*out_r += in_sample * self.gain;
					}
				}
			}
		} else {
			for input in inputs {
				assert!(!input.stereo(), "Trying to mix stereo signal with mono MixerNode");

				for (out_sample, &in_sample) in output.iter_mut().zip(input.iter()) {
					*out_sample += in_sample * self.gain;
				}
			}
		}
	}
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
	fn has_stereo_output(&self) -> bool { true }

	fn process(&mut self, _eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
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



pub struct OscillatorNode {
	// parameter
	freq: f32,

	// state
	phase: f32,
}


impl OscillatorNode {
	pub fn new(freq: f32) -> OscillatorNode {
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



pub struct WidenNode;

impl WidenNode {
	pub fn new() -> WidenNode { WidenNode }
}

impl Node for WidenNode {
	fn has_stereo_output(&self) -> bool { true }

	fn process(&mut self, _eval_ctx: &EvaluationContext, inputs: &[&IntermediateBuffer], output: &mut IntermediateBuffer) {
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