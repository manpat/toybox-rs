use crate::prelude::*;
use crate::audio::*;



pub trait NodeBuilder<const CHANNELS: usize> : 'static + Send + Sync + Sized {
	type ProcessState<'eval>;

	fn start_process<'eval>(&mut self, _: &EvaluationContext<'eval>) -> Self::ProcessState<'eval>;
	fn generate_frame(&mut self, _: &mut Self::ProcessState<'_>) -> [f32; CHANNELS];

	fn is_finished(&self, _: &EvaluationContext<'_>) -> bool { false }


	fn gain(self, gain: f32) -> GainNode<Self> {
		GainNode { inner: self, gain }
	}

	fn envelope(self, attack: f32, release: f32) -> EnvelopeNode<Self> {
		EnvelopeNode {
			inner: self,
			attack,
			sound_length: attack + release,

			time: 0.0,
		}
	}
}

pub trait MonoNodeBuilder : NodeBuilder<1> {
	fn build(self) -> BuiltMonoNode<Self> {
		BuiltMonoNode { node: self }
	}

	fn widen(self) -> WidenNode<Self> {
		WidenNode { inner: self }
	}
}

pub trait StereoNodeBuilder : NodeBuilder<2> {
	fn build(self) -> BuiltStereoNode<Self> {
		BuiltStereoNode { node: self }
	}
}

impl<T> MonoNodeBuilder for T where T: NodeBuilder<1> {}
impl<T> StereoNodeBuilder for T where T: NodeBuilder<2> {}



pub struct BuiltMonoNode<N: MonoNodeBuilder> {
	node: N,
}

impl<N: MonoNodeBuilder> Node for BuiltMonoNode<N> {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { false }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Source }

	fn finished_playing(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.node.is_finished(eval_ctx)
	}

	#[instrument(skip_all, name = "BuiltMonoNode::process")]
	fn process(&mut self, ProcessContext{output, eval_ctx, ..}: ProcessContext<'_>) {
		let mut state = self.node.start_process(eval_ctx);

		for frame in output.iter_mut() {
			*frame = self.node.generate_frame(&mut state)[0];
		}
	}
}


pub struct BuiltStereoNode<N: StereoNodeBuilder> {
	node: N,
}

impl<N: StereoNodeBuilder> Node for BuiltStereoNode<N> {
	fn has_stereo_output(&self, _: &EvaluationContext<'_>) -> bool { true }

	fn node_type(&self, _: &EvaluationContext<'_>) -> NodeType { NodeType::Source }

	fn finished_playing(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.node.is_finished(eval_ctx)
	}

	#[instrument(skip_all, name = "BuiltStereoNode::process")]
	fn process(&mut self, ProcessContext{output, eval_ctx, ..}: ProcessContext<'_>) {
		let mut state = self.node.start_process(eval_ctx);

		for frame in output.array_chunks_mut() {
			*frame = self.node.generate_frame(&mut state);
		}
	}
}



pub struct OscillatorGenerator {
	freq: f32,
	phase: f32,
}

impl OscillatorGenerator {
	pub fn new(freq: f32) -> OscillatorGenerator {
		OscillatorGenerator { freq, phase: 0.0 }
	}
}

impl NodeBuilder<1> for OscillatorGenerator {
	type ProcessState<'eval> = f32;

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> f32 {
		self.phase %= TAU;
		TAU * self.freq / eval_ctx.sample_rate
	}

	#[inline]
	fn generate_frame(&mut self, frame_period: &mut f32) -> [f32; 1] {
		let value = self.phase.sin();
		self.phase += *frame_period;
		[value]
	}
}


pub struct GainNode<N> {
	inner: N,
	gain: f32,
}

impl<N, const CHANNELS: usize> NodeBuilder<CHANNELS> for GainNode<N>
	where N: NodeBuilder<CHANNELS>
{
	type ProcessState<'eval> = N::ProcessState<'eval>;

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		self.inner.start_process(eval_ctx)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self, state: &mut Self::ProcessState<'_>) -> [f32; CHANNELS] {
		self.inner.generate_frame(state).map(|c| c * self.gain)
	}
}



pub struct WidenNode<N>
	where N: MonoNodeBuilder
{
	inner: N,
}

impl<N> NodeBuilder<2> for WidenNode<N>
	where N: MonoNodeBuilder
{
	type ProcessState<'eval> = N::ProcessState<'eval>;

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		self.inner.start_process(eval_ctx)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self, state: &mut Self::ProcessState<'_>) -> [f32; 2] {
		let [value] = self.inner.generate_frame(state);
		[value; 2]
	}
}


pub struct EnvelopeNode<N> {
	inner: N,

	attack: f32,
	sound_length: f32,

	time: f32,
}

impl<N, const CHANNELS: usize> NodeBuilder<CHANNELS> for EnvelopeNode<N>
	where N: NodeBuilder<CHANNELS>
{
	type ProcessState<'eval> = (f32, N::ProcessState<'eval>);

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		(1.0 / eval_ctx.sample_rate, self.inner.start_process(eval_ctx))
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.time > self.sound_length || self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self, state: &mut Self::ProcessState<'_>) -> [f32; CHANNELS] {
		let attack = (self.time / self.attack).min(1.0);
		let release = (1.0 - (self.time - self.attack) / (self.sound_length - self.attack)).powi(8);
		let envelope = (attack*release).clamp(0.0, 1.0);

		self.time += state.0;

		self.inner.generate_frame(&mut state.1).map(|c| c * envelope)
	}
}
