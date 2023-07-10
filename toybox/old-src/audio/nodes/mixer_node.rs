use crate::prelude::*;
use crate::audio::*;


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
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { self.stereo }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	#[instrument(skip_all, name = "MixerNode::process")]
	fn process(&mut self, ProcessContext{inputs, output, ..}: ProcessContext<'_>) {
		use std::simd::Simd;

		assert!(output.stereo() == self.stereo);

		let (first_input, remaining_inputs) = match inputs.split_first() {
			Some(pair) => pair,
			None => {
				// Zero buffer and bail if no inputs are connected.
				output.fill(0.0);
				return
			}
		};

		// Copy the first input to output to save the cost of zeroing the buffer.
		match (first_input.stereo(), output.stereo()) {
			(true, true) | (false, false) => {
				output.as_simd_mut().copy_from_slice(first_input.as_simd());
			}

			(false, true) => {
				for (out_sample, in_sample) in output.as_simd_mut().iter_mut().zip(first_input.iter_simd_widen()) {
					*out_sample = in_sample;
				}
			}

			(true, false) => panic!("Trying to mix stereo signal with mono MixerNode")
		}

		// Accumulate the rest of the inputs, widening if necessary.
		for input in remaining_inputs {
			match (input.stereo(), output.stereo()) {
				(true, true) | (false, false) => {
					for (out_sample, &in_sample) in output.as_simd_mut().iter_mut().zip(input.as_simd().iter()) {
						*out_sample += in_sample;
					}
				}

				(false, true) => {
					for (out_sample, in_sample) in output.as_simd_mut().iter_mut().zip(input.iter_simd_widen()) {
						*out_sample += in_sample;
					}
				}

				(true, false) => panic!("Trying to mix stereo signal with mono MixerNode")
			}
		}

		// Finally apply gain to the accumulated samples.
		let gain_simd = Simd::splat(self.gain);
		for out_sample_simd in output.as_simd_mut() {
			*out_sample_simd *= gain_simd;
		}
	}
}