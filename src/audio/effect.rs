use crate::prelude::*;
use crate::audio::*;
use super::{EvaluationContext, NodeBuilder};


pub trait Effect {
	fn start_process(&mut self, sample_dt: f32) {}
	fn next(&mut self, input: f32) -> f32;
}

pub struct EffectNode<N, E> {
	inner: N,
	effect: E,
}

impl<N, E> EffectNode<N, E> {
	pub fn new(inner: N, effect: E) -> Self {
		EffectNode {
			inner,
			effect,
		}
	}
}

impl<N, E> NodeBuilder<1> for EffectNode<N, E>
	where N: NodeBuilder<1>
		, E: Effect + Sync + Send + 'static
{
	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
		self.effect.start_process(eval_ctx.sample_dt);
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





pub struct LowPass {
	cutoff: f32,
	coefficient: f32,
	prev_value: f32,
}

impl LowPass {
	pub fn new(cutoff: f32) -> Self {
		LowPass {
			cutoff,
			coefficient: 1.0,
			prev_value: 0.0,
		}
	}
}

impl Effect for LowPass {
	fn start_process(&mut self, dt: f32) {
		self.coefficient = dt / (dt + 1.0 / (TAU * self.cutoff));
	}

	fn next(&mut self, input: f32) -> f32 {
		self.prev_value = self.coefficient.lerp(self.prev_value, input);
		self.prev_value
	}
}




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
	fn start_process(&mut self, dt: f32) {
		let rc = 1.0 / (TAU * self.cutoff);
		self.coefficient = rc / (rc + dt);
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