// https://splice.com/blog/dynamic-game-audio-mix/
// https://www.youtube.com/watch?v=UuqcgQxpfO8

pub mod runtime;

pub mod nodes;
pub mod node_builder;
pub mod generator;


pub use runtime::system::{AudioSystem, SoundId, EvaluationContext};
pub use runtime::node_graph::NodeId;
pub use runtime::scratch_buffer::{ScratchBuffer};

pub use nodes::{NodeType, Node, ProcessContext};
pub use node_builder::*;