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

	fn envelope<E: Envelope>(self, envelope: E) -> EnvelopeNode<Self, E> {
		EnvelopeNode::new(self, envelope)
	}
}

pub trait MonoNodeBuilder : NodeBuilder<1> {
	fn build(self) -> BuiltMonoNode<Self> {
		BuiltMonoNode { node: self }
	}

	fn widen(self) -> WidenNode<Self> {
		WidenNode { inner: self }
	}

	fn low_pass(self, cutoff: f32) -> LowPassNode<Self> {
		LowPassNode {
			inner: self,
			cutoff,
			prev_value: 0.0,
		}
	}

	fn high_pass(self, cutoff: f32) -> HighPassNode<Self> {
		HighPassNode {
			inner: self,
			cutoff,
			prev_value_diff: 0.0,
		}
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



use std::num::Wrapping;

// https://www.musicdsp.org/en/latest/Synthesis/216-fast-whitenoise-generator.html
pub struct NoiseGenerator {
	x1: Wrapping<i32>,
	x2: Wrapping<i32>,
}

impl NoiseGenerator {
	pub fn new() -> NoiseGenerator {
		#[allow(overflowing_literals)]
		NoiseGenerator {
			x1: Wrapping(0x67452301i32),
			x2: Wrapping(0xefcdab89i32),
		}
	}
}

impl NodeBuilder<1> for NoiseGenerator {
	type ProcessState<'eval> = ();

	fn start_process<'eval>(&mut self, _: &EvaluationContext<'eval>) {}

	#[inline]
	fn generate_frame(&mut self, _: &mut ()) -> [f32; 1] {
		#[allow(overflowing_literals)]
		const SCALE: f32 = 2.0 / 0xffffffff as f32;

		self.x2 += self.x1;
		self.x1 ^= self.x2;
		
		[(SCALE * self.x2.0 as f32).clamp(-1.0, 1.0)]
	}
}



pub struct LowPassNode<N> {
	inner: N,
	cutoff: f32,
	prev_value: f32,
}

impl<N> NodeBuilder<1> for LowPassNode<N>
	where N: MonoNodeBuilder
{
	type ProcessState<'eval> = (N::ProcessState<'eval>, f32);

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		let dt = 1.0 / eval_ctx.sample_rate;
		let a = dt / (dt + 1.0 / (TAU * self.cutoff));

		(self.inner.start_process(eval_ctx), a)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self, (state, a): &mut Self::ProcessState<'_>) -> [f32; 1] {
		let [new_value] = self.inner.generate_frame(state);
		self.prev_value = a.lerp(self.prev_value, new_value);
		[self.prev_value]
	}
}


pub struct HighPassNode<N> {
	inner: N,
	cutoff: f32,
	prev_value_diff: f32,
}

impl<N> NodeBuilder<1> for HighPassNode<N>
	where N: MonoNodeBuilder
{
	type ProcessState<'eval> = (N::ProcessState<'eval>, f32);

	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
		let dt = 1.0 / eval_ctx.sample_rate;
		let rc = 1.0 / (TAU * self.cutoff);
		let a = rc / (rc + dt);

		(self.inner.start_process(eval_ctx), a)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self, (state, a): &mut Self::ProcessState<'_>) -> [f32; 1] {
		let [new_value] = self.inner.generate_frame(state);
		let result = *a * (self.prev_value_diff + new_value);
		self.prev_value_diff = result - new_value;
		[result]
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




// TODO(pat.m): add sum and multiply combinators on (N, ...) that create AddNode and MultiplyNode


macro_rules! impl_nodebuilder_for_tuple {
	($($idx:tt -> $ty:ident),*) => {
		impl< $($ty),* , const CHANNELS: usize> NodeBuilder<CHANNELS> for ( $($ty,)* )
			where $(
				$ty : NodeBuilder<CHANNELS>
			),*
		{
			type ProcessState<'eval> = ( $( $ty::ProcessState<'eval>, )* );

			fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) -> Self::ProcessState<'eval> {
				( $( self.$idx.start_process(eval_ctx), )* )
			}

			fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
				$(
					if !self.$idx.is_finished(eval_ctx) {
						return false
					}
				)*

				return true
			}

			#[inline]
			fn generate_frame(&mut self, state: &mut Self::ProcessState<'_>) -> [f32; CHANNELS] {
				let frames = [
					$( self.$idx.generate_frame(&mut state.$idx), )*
				];

				frames.into_iter()
					.fold([0.0; CHANNELS], |acc, frame| {
						acc.zip(frame).map(|(c0, c1)| c0 + c1)
					})
			}
		}
	}
}

impl_nodebuilder_for_tuple!(0 -> N0);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3, 4 -> N4);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3, 4 -> N4, 5 -> N5);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3, 4 -> N4, 5 -> N5, 6 -> N6);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3, 4 -> N4, 5 -> N5, 6 -> N6, 7 -> N7);
impl_nodebuilder_for_tuple!(0 -> N0, 1 -> N1, 2 -> N2, 3 -> N3, 4 -> N4, 5 -> N5, 6 -> N6, 7 -> N7, 8 -> N8);


// TODO(pat.m): ComposeNode
// impl<A, B> ComposeNode<A, B>
// {
// 	type ProcessState<'eval> = ( A::ProcessState<'eval>, B::ProcessState<'eval> );
// 
// 	fn generate_frame(&mut self, ..) -> Frame {
// 		let inner_frame = self.inner.generate_frame(..);
// 		outer.feed(inner_frame);
// 	}
// }
// 