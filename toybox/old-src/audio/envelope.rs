use crate::prelude::*;
use crate::audio::*;
use super::{EvaluationContext, NodeBuilder};


pub trait Envelope : Sized {
	fn is_finished(&self) -> bool;
	fn next(&mut self, dt: f32) -> f32;

	fn to_parameter(self) -> parameter::EnvelopeParameter<Self> {
		parameter::EnvelopeParameter(self, 0.0)
	}
}


#[derive(Copy, Clone, Debug)]
pub struct Ramp {
	from: f32,
	to: f32,
	duration: f32,

	phase: f32,
}

impl Ramp {
	pub fn new(duration: f32, from: f32, to: f32) -> Ramp {
		Ramp {
			from,
			to,
			duration,
			phase: 0.0,
		}
	}

	pub fn up(duration: f32) -> Ramp {
		Ramp::new(duration, 0.0, 1.0)
	}

	pub fn down(duration: f32) -> Ramp {
		Ramp::new(duration, 1.0, 0.0)
	}
}

impl Envelope for Ramp {
	fn is_finished(&self) -> bool {
		self.phase > 1.0
	}

	fn next(&mut self, dt: f32) -> f32 {
		let phase = self.phase;
		self.phase += dt / self.duration;

		phase.clamp(0.0, 1.0)
			.lerp(self.from, self.to)
	}
}


#[derive(Copy, Clone, Debug)]
pub struct AR {
	attack: f32,
	release: f32,

	time: f32,
}

impl AR {
	pub fn new(attack: f32, release: f32) -> AR {
		AR {
			attack,
			release,
			time: 0.0,
		}
	}

	pub fn exp(self, exp: f32) -> ExpAR {
		self.exp2(1.0 / exp, exp)
	}

	pub fn exp2(self, atk: f32, rel: f32) -> ExpAR {
		ExpAR::new(self.attack, self.release, atk, rel)
	}
}

impl Envelope for AR {
	fn is_finished(&self) -> bool {
		self.time > self.attack + self.release
	}

	fn next(&mut self, dt: f32) -> f32 {
		let time = self.time;
		self.time += dt;

		if time < self.attack {
			(time / self.attack).max(0.0)
		} else {
			(1.0 - (time - self.attack) / self.release).max(0.0)
		}
	}
}



#[derive(Copy, Clone, Debug)]
pub struct ExpAR {
	attack: f32,
	release: f32,
	attack_exponent: f32,
	release_exponent: f32,

	time: f32,
}

impl ExpAR {
	pub fn new(attack: f32, release: f32, attack_exponent: f32, release_exponent: f32) -> ExpAR {
		ExpAR {
			attack,
			release,
			attack_exponent,
			release_exponent,
			time: 0.0,
		}
	}
}

impl Envelope for ExpAR {
	fn is_finished(&self) -> bool {
		self.time > self.attack + self.release
	}

	fn next(&mut self, dt: f32) -> f32 {
		let time = self.time;
		self.time += dt;

		if time < self.attack {
			let linear = (time / self.attack).max(0.0);
			linear.powf(self.attack_exponent)
		} else {
			(1.0 - ((time - self.attack) / self.release)).max(0.0).powf(self.release_exponent)
		}
	}
}







pub struct EnvelopeNode<N, E> {
	inner: N,
	envelope: E,

	sample_dt: f32,
}

impl<N, E> EnvelopeNode<N, E> {
	pub fn new(inner: N, envelope: E) -> Self {
		EnvelopeNode {
			inner,
			envelope,

			sample_dt: 0.0,
		}
	}
}


impl<N, E, const CHANNELS: usize> NodeBuilder<CHANNELS> for EnvelopeNode<N, E>
	where N: NodeBuilder<CHANNELS>
		, E: Envelope + Sync + Send + 'static
{
	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
		self.sample_dt = eval_ctx.sample_dt;
		self.inner.start_process(eval_ctx);
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.envelope.is_finished() || self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self) -> [f32; CHANNELS] {
		let envelope = self.envelope.next(self.sample_dt);
		self.inner.generate_frame().map(|c| c * envelope)
	}
}

