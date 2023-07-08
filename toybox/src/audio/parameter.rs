use crate::prelude::*;
use crate::audio::*;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;



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




#[derive(Clone)]
pub struct AtomicFloatParameter(Arc<AtomicU32>);

impl AtomicFloatParameter {
	pub fn new(initial_value: f32) -> Self {
		AtomicFloatParameter(Arc::new(AtomicU32::new(initial_value.to_bits())))
	}

	pub fn write(&self, value: f32) {
		self.0.store(value.to_bits(), Ordering::Relaxed);
	}

	pub fn read(&self) -> f32 {
		f32::from_bits(self.0.load(Ordering::Relaxed))
	}
}

impl audio::FloatParameter for AtomicFloatParameter {
	const AUDIO_RATE: bool = false;

	fn eval(&mut self) -> f32 {
		self.read()
	}
}