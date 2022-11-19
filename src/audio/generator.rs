use crate::prelude::*;
use super::{EvaluationContext, NodeBuilder, FloatParameter};


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



pub struct GeneratorNode<F, FP> {
	phase: f32,
	freq: FP,
	phase_dt: f32, // TODO(pat.m): how do I make this dynamic
	f: F,
}


impl<F, FP> GeneratorNode<F, FP>
	where F: FnMut(f32) -> f32
		, FP: FloatParameter
{
	pub fn new(freq: FP, f: F) -> Self {
		GeneratorNode {
			phase: 0.0,
			freq,
			phase_dt: 0.0,
			f
		}
	}
}

impl<FP> GeneratorNode<fn(f32) -> f32, FP>
	where FP: FloatParameter
{
	pub fn new_sine(freq: FP) -> Self {
		GeneratorNode::new(freq, sine_wave)
	}

	pub fn new_saw(freq: FP) -> Self {
		GeneratorNode::new(freq, saw_wave)
	}

	pub fn new_triangle(freq: FP) -> Self {
		GeneratorNode::new(freq, triangle_wave)
	}

	pub fn new_square(freq: FP) -> Self {
		GeneratorNode::new(freq, square_wave)
	}

	pub fn new_pulse(freq: FP, width: f32) -> GeneratorNode<impl FnMut(f32) -> f32, FP> {
		GeneratorNode::new(freq, move |ph| pulse_wave(ph, width))
	}
}



impl<F, FP> NodeBuilder<1> for GeneratorNode<F, FP>
	where F: FnMut(f32) -> f32 + Send + Sync + 'static
		, FP: FloatParameter
{
	fn start_process<'eval>(&mut self, ctx: &EvaluationContext<'eval>) {
		self.phase = self.phase.fract();

		self.freq.update(ctx);

		if FP::AUDIO_RATE {
			self.phase_dt = ctx.sample_dt;
		} else {
			self.phase_dt = ctx.sample_dt * self.freq.eval();
		}
	}

	#[inline]
	fn generate_frame(&mut self) -> [f32; 1] {
		let value = (self.f)(self.phase);

		let phase_dt = match FP::AUDIO_RATE {
			false => self.phase_dt,
			true => self.phase_dt * self.freq.eval(),
		};

		self.phase += phase_dt;
		[value]
	}
}




use std::num::Wrapping;

// https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
pub struct Noise {
	x1: Wrapping<i32>,
	x2: Wrapping<i32>,
}

impl Noise {
	pub fn new() -> Noise {
		#[allow(overflowing_literals)]
		Noise {
			x1: Wrapping(0x67452301i32),
			x2: Wrapping(0xefcdab89i32),
		}
	}

	pub fn next(&mut self) -> f32 {
		#[allow(overflowing_literals)]
		const SCALE: f32 = 2.0 / 0xffffffff as f32;

		self.x2 += self.x1;
		self.x1 ^= self.x2;
		
		(SCALE * self.x2.0 as f32).clamp(-1.0, 1.0)
	}
}

impl NodeBuilder<1> for Noise {
	#[inline]
	fn generate_frame(&mut self) -> [f32; 1] {
		[self.next()]
	}
}