// https://splice.com/blog/dynamic-game-audio-mix/
// https://www.youtube.com/watch?v=UuqcgQxpfO8

pub mod runtime;

pub mod effect;
pub mod envelope;
pub mod generator;
pub mod node_builder;
pub mod nodes;
pub mod parameter;
pub mod util;


pub use runtime::system::{AudioSystem, SoundId, EvaluationContext};
pub use runtime::node_graph::NodeId;
pub use runtime::scratch_buffer::{ScratchBuffer};

pub use nodes::{NodeType, Node, ProcessContext};
pub use node_builder::*;

pub use envelope::{Envelope, EnvelopeNode};
pub use effect::{Effect, EffectStage, EffectNode};
pub use parameter::FloatParameter;