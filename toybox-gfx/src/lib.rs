use toybox_host as host;

use host::prelude::*;
use host::gl;

pub mod core;
pub use crate::core::*;

pub mod prelude {
    pub use crate::host::gl;
    pub use crate::core::*;
}


pub struct System {
    pub core: core::Core,
    pub resource_manager: ResourceManager,
    pub encoder: Encoder,
}



// Create/Destroy api for gpu resources
// Load/Cache resources from disk
// Render target/FBO/temporary image cahage
//  - cache of images for use as single-frame render targets, automatically resized
//  - cache of images for use as single-frame image resources -  fixed size
//  - cache of FBOs for render passes
// Shader cache
pub struct ResourceManager {}


// Encodes per-frame commands, organised into passes/command groups
pub struct Encoder {

}

impl Encoder {
    pub fn command_group(&mut self, _id: &str) -> CommandGroupEncoder<'_> {
        todo!()
    }
}


pub struct CommandGroupEncoder<'g> {
    group: &'g mut CommandGroup,
}


// 
pub struct CommandGroup {
    label: String,

    commands: Vec<Command>,

    shared_bindings: BindingDescription,
}


pub enum Command {
    Draw {
        args: DrawArgs,
        bindings: BindingDescription,
    },

    Compute {
        args: DispatchArgs,
        bindings: BindingDescription,
    },

    ClearBuffer,
    ClearTexture,

    CopyBuffer,
    CopyTexture,

    DebugMessage { label: String, },
}


pub struct UploadHeap {}


pub struct BindingDescription {}


pub struct DrawArgs {}
pub struct DispatchArgs {}
