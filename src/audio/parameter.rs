use crate::prelude::*;
use crate::audio::*;



pub trait FloatParameter: Sync + Send + 'static {
	const AUDIO_RATE: bool;

	fn update(&mut self, _: &EvaluationContext<'_>) {}
	fn eval(&mut self) -> f32;
}


impl FloatParameter for f32 {
	const AUDIO_RATE: bool = false;

	fn eval(&mut self) -> f32 {
		*self
	}
}




pub struct NodeBuilderParameter<N> (pub N);

impl<N> FloatParameter for NodeBuilderParameter<N>
	where N: MonoNodeBuilder
{
	const AUDIO_RATE: bool = true;

	fn update(&mut self, eval_ctx: &EvaluationContext<'_>) {
		self.0.start_process(eval_ctx);
	}

	fn eval(&mut self) -> f32 {
		self.0.generate_frame()[0]
	}
}




pub struct EnvelopeParameter<E>(pub E, pub f32);

impl<E> FloatParameter for EnvelopeParameter<E>
	where E: Envelope + Sync + Send + 'static
{
	const AUDIO_RATE: bool = true;

	fn update(&mut self, eval_ctx: &EvaluationContext<'_>) {
		self.1 = eval_ctx.sample_dt;
	}

	fn eval(&mut self) -> f32 {
		self.0.next(self.1)
	}
}