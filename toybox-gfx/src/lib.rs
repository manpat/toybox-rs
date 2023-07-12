use toybox_host as host;

use host::prelude::*;
use host::gl;


pub mod prelude {
    pub use crate::host::gl;
}


pub struct Core {
    surface: host::Surface,
    gl_context: host::GlContext,
    pub gl: gl::Gl,
}

impl Core {
    pub fn new(surface: host::Surface, gl_context: host::GlContext, gl: gl::Gl)
        -> Core
    {
        Core {
            surface,
            gl_context,
            gl,
        }
    }

    pub fn finalize_frame(&self) {
        self.surface.swap_buffers(&self.gl_context).unwrap();
    }
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

    // For debugging - does renderdoc show this?
    DebugMessage { label: String, },
}


pub struct UploadHeap {}


pub struct BindingDescription {}


pub struct DrawArgs {}
pub struct DispatchArgs {}
