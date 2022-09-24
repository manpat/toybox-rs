// https://splice.com/blog/dynamic-game-audio-mix/
// https://www.youtube.com/watch?v=UuqcgQxpfO8

pub mod system;
pub mod nodes;
pub mod intermediate_buffer;
pub mod node_builder;

mod node_graph;
mod ringbuffer;


pub use system::{AudioSystem, SoundId};
pub use node_graph::NodeId;

pub use system::{EvaluationContext};
pub use intermediate_buffer::{IntermediateBuffer};
pub use nodes::{NodeType, Node, ProcessContext};

pub use node_builder::*;