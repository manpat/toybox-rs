use crate::prelude::*;
use crate::audio::*;
use super::{EvaluationContext, NodeBuilder};


pub trait Effect {
	fn start_process(&mut self, _: &EvaluationContext<'_>) {}
	fn next(&mut self, input: f32) -> f32;
}



fn low_pass_coefficient(dt: f32, cutoff: f32) -> f32 {
	let rc = 1.0 / (TAU * cutoff.max(0.1));
	dt / (dt + rc)
}


#[derive(Clone)]
pub struct LowPass<P> {
	cutoff: P,
	coefficient: f32,
	prev_value: f32,
}

impl<P: FloatParameter> LowPass<P> {
	pub fn new(cutoff: P) -> Self {
		LowPass {
			cutoff,
			coefficient: 1.0,
			prev_value: 0.0,
		}
	}
}

impl<P: FloatParameter> Effect for LowPass<P> {
	fn start_process(&mut self, eval_ctx: &EvaluationContext<'_>) {
		self.cutoff.update(eval_ctx);

		self.coefficient = match P::AUDIO_RATE {
			true => eval_ctx.sample_dt,
			false => low_pass_coefficient(eval_ctx.sample_dt, self.cutoff.eval()),
		};
	}

	fn next(&mut self, input: f32) -> f32 {
		let coefficient = match P::AUDIO_RATE {
			true => low_pass_coefficient(self.coefficient, self.cutoff.eval()),
			false => self.coefficient,
		};

		self.prev_value = coefficient.lerp(self.prev_value, input);
		self.prev_value
	}
}




fn resonant_low_pass_coefficients(dt: f32, cutoff: f32, q: f32) -> (f32, f32) {
	let q = q.clamp(0.0, 1.0);
	let cutoff = cutoff.max(1.0);
	let coeff = 2.0 * (PI * cutoff * dt).sin();
	let fb_coeff = q + q / (1.0 - coeff);
	(coeff, fb_coeff)
}


#[derive(Clone)]
pub struct ResonantLowPass<CP, QP> {
	cutoff: CP,
	q: QP,

	coefficient: f32,
	fb_coefficient: f32,
	prev_0: f32,
	prev_1: f32,
}

impl<CP, QP> ResonantLowPass<CP, QP> {
	pub fn new(cutoff: CP, q: QP) -> Self {
		ResonantLowPass {
			cutoff,
			q,

			coefficient: 0.0,
			fb_coefficient: 0.0,
			prev_0: 0.0,
			prev_1: 0.0,
		}
	}
}

impl<CP, QP> Effect for ResonantLowPass<CP, QP>
	where CP: FloatParameter
		, QP: FloatParameter
{
	fn start_process(&mut self, eval_ctx: &EvaluationContext<'_>) {
		self.cutoff.update(eval_ctx);
		self.q.update(eval_ctx);

		(self.coefficient, self.fb_coefficient) = match CP::AUDIO_RATE || QP::AUDIO_RATE {
			true => (eval_ctx.sample_dt, 0.0),
			false => resonant_low_pass_coefficients(eval_ctx.sample_dt, self.cutoff.eval(), self.q.eval()),
		};
	}

	fn next(&mut self, input: f32) -> f32 {
		let (coefficient, fb_coefficient) = match CP::AUDIO_RATE || QP::AUDIO_RATE {
			true => resonant_low_pass_coefficients(self.coefficient, self.cutoff.eval(), self.q.eval()),
			false => (self.coefficient, self.fb_coefficient),
		};

		let hp = input - self.prev_0;
		let bp = self.prev_0 - self.prev_1;

		self.prev_0 += coefficient * (hp + fb_coefficient * bp);
		self.prev_1 += coefficient * (self.prev_0 - self.prev_1);

		self.prev_1
	}
}







fn high_pass_coefficient(dt: f32, cutoff: f32) -> f32 {
	let rc = 1.0 / (TAU * cutoff.max(0.1));
	rc / (dt + rc)
}


#[derive(Clone)]
pub struct HighPass {
	cutoff: f32,
	coefficient: f32,
	prev_value_diff: f32,
}

impl HighPass {
	pub fn new(cutoff: f32) -> Self {
		HighPass {
			cutoff,
			coefficient: 1.0,
			prev_value_diff: 0.0,
		}
	}
}

impl Effect for HighPass {
	fn start_process(&mut self, eval_ctx: &EvaluationContext<'_>) {
		self.coefficient = high_pass_coefficient(eval_ctx.sample_dt, self.cutoff);
	}

	fn next(&mut self, input: f32) -> f32 {
		let result = self.coefficient * (self.prev_value_diff + input);
		self.prev_value_diff = result - input;
		result
	}
}





impl<F> Effect for F
	where F: FnMut(f32) -> f32
{
	fn next(&mut self, input: f32) -> f32 {
		(self)(input)
	}
}



pub struct EffectStage<N, E> {
	inner: N,
	effect: E,
}

impl<N, E> EffectStage<N, E> {
	pub fn new(inner: N, effect: E) -> Self {
		EffectStage {
			inner,
			effect,
		}
	}
}

impl<N, E> NodeBuilder<1> for EffectStage<N, E>
	where N: NodeBuilder<1>
		, E: Effect + Sync + Send + 'static
{
	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
		self.effect.start_process(eval_ctx);
		self.inner.start_process(eval_ctx);
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self) -> [f32; 1] {
		let [value] = self.inner.generate_frame();
		[self.effect.next(value)]
	}
}



pub struct EffectNode<E> {
	effect: E,
}

impl<E> EffectNode<E> {
	pub fn new(effect: E) -> Self {
		EffectNode {
			effect,
		}
	}
}

impl<E> Node for EffectNode<E>
	where E: Effect + Sync + Send + 'static
{
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	fn process(&mut self, ProcessContext{inputs, output, eval_ctx, ..}: ProcessContext<'_>) {
		assert!(!output.stereo());

		let Some(input) = inputs.get(0) else {
			output.fill(0.0);
			return
		};

		assert!(!input.stereo());

		self.effect.start_process(eval_ctx);

		for (out_sample, &in_sample) in output.iter_mut().zip(input.iter()) {
			*out_sample = self.effect.next(in_sample);
		}
	}
}


pub struct StereoEffectNode<E> {
	left: E,
	right: E,
}

impl<E: Clone> StereoEffectNode<E> {
	pub fn new(effect: E) -> Self {
		StereoEffectNode {
			left: effect.clone(),
			right: effect,
		}
	}
}

impl<E> Node for StereoEffectNode<E>
	where E: Effect + Sync + Send + 'static
{
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Effect }

	fn process(&mut self, ProcessContext{inputs, output, eval_ctx, ..}: ProcessContext<'_>) {
		assert!(output.stereo());

		let Some(input) = inputs.get(0) else {
			output.fill(0.0);
			return
		};

		assert!(input.stereo());

		self.left.start_process(eval_ctx);
		self.right.start_process(eval_ctx);

		for ([out_l, out_r], &[in_l, in_r]) in output.array_chunks_mut().zip(input.array_chunks()) {
			*out_l = self.left.next(in_l);
			*out_r = self.right.next(in_r);
		}
	}
}
