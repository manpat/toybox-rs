use crate::prelude::*;
use crate::audio::*;

pub struct CompressorNode {
	attack: f32,
	release: f32,
	threshold_db: f32,
	ratio: f32,

	// signal_dc: f32,
	envelope: f32,
}

impl CompressorNode {
	pub fn new(attack: f32, release: f32, threshold_db: f32, ratio: f32) -> Self {
		CompressorNode {
			attack,
			release,
			threshold_db,
			ratio,

			// signal_dc: 0.0,
			envelope: 0.0,
		}
	}
}

impl Node for CompressorNode {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	#[instrument(skip_all, name = "CompressorNode::process")]
	fn process(&mut self, ProcessContext{inputs, output, eval_ctx}: ProcessContext<'_>) {
		let Some(input) = inputs.first() else {
			output.fill(0.0);
			return;
		};

		let attack  = 1.0 - (-1.0 / (self.attack * eval_ctx.sample_rate)).exp();
		let release = 1.0 - (-1.0 / (self.release * eval_ctx.sample_rate)).exp();

		for ([in_l, in_r], [out_l, out_r]) in input.array_chunks().zip(output.array_chunks_mut()) {
			let max_rectified = in_l.abs().max(in_r.abs());

			let key_db = linear_to_db(max_rectified + DC_OFFSET);
			let over_db = (key_db - self.threshold_db).max(0.0);

			if over_db > self.envelope {
				self.envelope = attack.lerp(self.envelope, over_db);
			} else {
				self.envelope = release.lerp(self.envelope, over_db);
			}

			let gain_db = self.envelope * (self.ratio - 1.0);
			let gain = db_to_linear(gain_db);

			*out_l = (in_l * gain).clamp(-1.0, 1.0);
			*out_r = (in_r * gain).clamp(-1.0, 1.0);
		}
	}
}


const DC_OFFSET: f32 = 1.0E-25;


fn linear_to_db(lin: f32) -> f32 {
	lin.ln() * 20.0 / std::f32::consts::LN_10
}

fn db_to_linear(db: f32) -> f32 {
	(db * std::f32::consts::LN_10 / 20.0).exp()
}