use crate::prelude::*;
use crate::audio::*;



pub trait NodeBuilder<const CHANNELS: usize> : 'static + Send + Sync + Sized {
	fn start_process<'eval>(&mut self, _: &EvaluationContext<'eval>) {}
	fn generate_frame(&mut self) -> [f32; CHANNELS];

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

	fn effect<E: Effect>(self, effect: E) -> EffectNode<Self, E> {
		EffectNode::new(self, effect)
	}

	fn low_pass(self, cutoff: f32) -> EffectNode<Self, effect::LowPass> {
		self.effect(effect::LowPass::new(cutoff))
	}

	fn high_pass(self, cutoff: f32) -> EffectNode<Self, effect::HighPass> {
		self.effect(effect::HighPass::new(cutoff))
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
		self.node.start_process(eval_ctx);

		for frame in output.iter_mut() {
			*frame = self.node.generate_frame()[0];
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
		self.node.start_process(eval_ctx);

		for frame in output.array_chunks_mut() {
			*frame = self.node.generate_frame();
		}
	}
}





pub struct GainNode<N> {
	inner: N,
	gain: f32,
}

impl<N, const CHANNELS: usize> NodeBuilder<CHANNELS> for GainNode<N>
	where N: NodeBuilder<CHANNELS>
{
	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
		self.inner.start_process(eval_ctx)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self) -> [f32; CHANNELS] {
		self.inner.generate_frame().map(|c| c * self.gain)
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
	fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
		self.inner.start_process(eval_ctx)
	}

	fn is_finished(&self, eval_ctx: &EvaluationContext<'_>) -> bool {
		self.inner.is_finished(eval_ctx)
	}

	#[inline]
	fn generate_frame(&mut self) -> [f32; 2] {
		let [value] = self.inner.generate_frame();
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
			fn start_process<'eval>(&mut self, eval_ctx: &EvaluationContext<'eval>) {
				$( self.$idx.start_process(eval_ctx); )*
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
			fn generate_frame(&mut self) -> [f32; CHANNELS] {
				let frames = [
					$( self.$idx.generate_frame(), )*
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