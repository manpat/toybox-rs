use crate::prelude::*;
use super::{EvaluationContext, NodeBuilder};


pub fn sine_wave(phase: f32) -> f32 {
	(phase * TAU).sin()
}

pub fn triangle_wave(phase: f32) -> f32 {
	let phase = phase.fract();
	if phase <= 0.5 {
		(phase - 0.25) * 4.0
	} else {
		(0.75 - phase) * 4.0
	}
}

pub fn saw_wave(phase: f32) -> f32 {
	phase.fract() * 2.0 - 1.0
}

pub fn square_wave(phase: f32) -> f32 {
	pulse_wave(phase, 0.5)
}

pub fn pulse_wave(phase: f32, width: f32) -> f32 {
	(phase.fract() - width).floor() * -2.0 - 1.0
}


// TODO(pat.m): noise gen



pub struct GeneratorNode<F> {
	phase: f32,
	phase_dt: f32, // TODO(pat.m): how do I make this dynamic
	f: F,
}


impl<F> GeneratorNode<F>
	where F: FnMut(f32) -> f32
{
	pub fn new(freq: f32, f: F) -> Self {
		GeneratorNode {
			phase: 0.0,
			phase_dt: freq,
			f
		}
	}
}

impl GeneratorNode<fn(f32) -> f32> {
	pub fn new_sine(freq: f32) -> Self {
		GeneratorNode::new(freq, sine_wave)
	}

	pub fn new_saw(freq: f32) -> Self {
		GeneratorNode::new(freq, saw_wave)
	}

	pub fn new_triangle(freq: f32) -> Self {
		GeneratorNode::new(freq, triangle_wave)
	}

	pub fn new_square(freq: f32) -> Self {
		GeneratorNode::new(freq, square_wave)
	}

	pub fn new_pulse(freq: f32, width: f32) -> GeneratorNode<impl FnMut(f32) -> f32> {
		GeneratorNode::new(freq, move |ph| pulse_wave(ph, width))
	}
}



impl<F> NodeBuilder<1> for GeneratorNode<F>
	where F: FnMut(f32) -> f32 + Send + Sync + 'static
{
	type ProcessState<'eval> = f32;

	fn start_process<'eval>(&mut self, ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		self.phase = self.phase.fract();
		ctx.sample_dt
	}

	#[inline]
	fn generate_frame(&mut self, sample_dt: &mut Self::ProcessState<'_>) -> [f32; 1] {
		let value = (self.f)(self.phase);
		self.phase += self.phase_dt * *sample_dt;
		[value]
	}
}

