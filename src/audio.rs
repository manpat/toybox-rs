// https://splice.com/blog/dynamic-game-audio-mix/
// https://www.youtube.com/watch?v=UuqcgQxpfO8

pub mod system;
pub mod nodes;
pub mod scratch_buffer;
pub mod node_builder;

mod node_graph;
mod execution_graph;
mod scratch_buffer_cache;
mod ringbuffer;


pub use system::{AudioSystem, SoundId};
pub use node_graph::NodeId;

pub use system::{EvaluationContext};
pub use scratch_buffer::{ScratchBuffer};
pub use nodes::{NodeType, Node, ProcessContext};

pub use node_builder::*;