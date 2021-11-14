// https://splice.com/blog/dynamic-game-audio-mix/
// https://www.youtube.com/watch?v=UuqcgQxpfO8

pub mod system;
pub mod nodes;
pub mod intermediate_buffer;

mod node_graph;
mod intermediate_buffer_cache;
mod ringbuffer;


pub const MAX_NODE_INPUTS: usize = 16;


pub use system::{AudioSystem, SoundId};
pub use node_graph::NodeId;

pub type SoundAssetID = ();
pub type SoundInstanceID = ();