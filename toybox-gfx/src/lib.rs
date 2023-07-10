use toybox_host as host;

use host::prelude::*;


pub mod prelude {
    pub use gl;
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