use crate::prelude::*;
use crate::audio::{
	system, system::EvaluationContext,
	intermediate_buffer::IntermediateBuffer,
	parameter::Parameter,
};



pub trait Node: 'static + Send + Sync {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool;
	fn finished_playing(&self, _: &EvaluationContext<'_>) -> bool { false }
	fn process(&mut self, _: ProcessContext<'_>);
}


pub struct ProcessContext<'ctx> {
	pub eval_ctx: &'ctx EvaluationContext<'ctx>,
	pub inputs: &'ctx [&'ctx IntermediateBuffer],
	pub output: &'ctx mut IntermediateBuffer,
}




pub struct MixerNode {
	// parameter
	gain: Parameter<f32>,
	stereo: bool,
}


impl MixerNode {
	pub fn new(gain: impl Into<Parameter<f32>>) -> MixerNode {
		MixerNode { gain: gain.into(), stereo: false }
	}

	pub fn new_stereo(gain: impl Into<Parameter<f32>>) -> MixerNode {
		MixerNode { gain: gain.into(), stereo: true }
	}
}

use std::mem::MaybeUninit;

impl Node for MixerNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { self.stereo }

	fn process(&mut self, ProcessContext{inputs, output, ..}: ProcessContext<'_>) {
		assert!(output.stereo() == self.stereo);

		output.fill(0.0);

		let mut gain_storage = [MaybeUninit::<f32>::uninit(); 2048];
		let gain_samples = if self.stereo { output.len()/2 } else { output.len() };

		let gains = std::iter::repeat_with(|| self.gain.get()).take(gain_samples);
		let gains = init_fixed_buffer_from_iterator(&mut gain_storage, gains);


		// let gain = self.gain.get();
		if self.stereo {
			for input in inputs {
				if input.stereo() {
					for (([out_l, out_r], &[in_l, in_r]), &gain) in output.array_chunks_mut::<2>().zip(input.array_chunks::<2>()).zip(gains) {
						*out_l += in_l * gain;
						*out_r += in_r * gain;
					}
				} else {
					for (([out_l, out_r], &in_sample), &gain) in output.array_chunks_mut::<2>().zip(input.iter()).zip(gains) {
						*out_l += in_sample * gain;
						*out_r += in_sample * gain;
					}
				}
			}
		} else {
			for input in inputs {
				assert!(!input.stereo(), "Trying to mix stereo signal with mono MixerNode");

				for ((out_sample, &in_sample), &gain) in output.iter_mut().zip(input.iter()).zip(gains) {
					*out_sample += in_sample * gain;
				}
			}
		}
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



pub struct OscillatorNode {
	// parameter
	freq: Parameter<f32>,

	// state
	phase: f32,
}


impl OscillatorNode {
	pub fn new(freq: impl Into<Parameter<f32>>) -> OscillatorNode {
		OscillatorNode {
			freq: freq.into(),
			phase: 0.0,
		}
	}
}

impl Node for OscillatorNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn process(&mut self, ProcessContext{eval_ctx, inputs, output}: ProcessContext<'_>) {
		assert!(inputs.is_empty());

		let frame_inc = TAU / eval_ctx.sample_rate;

		for out_sample in output.iter_mut() {
			let frame_period = self.freq.get() * frame_inc;

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
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

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

	fn finished_playing(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		let buffer = &eval_ctx.resources.get(self.sound_id);
		self.position >= buffer.len()
	}

	fn process(&mut self, ProcessContext{eval_ctx, inputs, output}: ProcessContext<'_>) {
		assert!(inputs.is_empty());
		assert!(!output.stereo());

		let buffer = &eval_ctx.resources.get(self.sound_id);

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


