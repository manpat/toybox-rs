use crate::prelude::*;

use toybox_host as host;
use host::prelude::*;


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


/// Debug
impl Core {
	pub fn push_debug_group(&self, message: &str) {
		let id = 0;

		unsafe {
			self.gl.PushDebugGroup(gl::DEBUG_SOURCE_APPLICATION, id, message.len() as i32, message.as_ptr() as *const _);
		}
	}

	pub fn pop_debug_group(&self) {
		unsafe {
			self.gl.PopDebugGroup();
		}
	}

	pub fn debug_marker(&self, message: &str) {
		let id = 0;

		unsafe {
			self.gl.DebugMessageInsert(
				gl::DEBUG_SOURCE_APPLICATION,
				gl::DEBUG_TYPE_MARKER,
				id,
				gl::DEBUG_SEVERITY_NOTIFICATION,
				message.len() as i32,
				message.as_ptr() as *const _
			);
		}
	}
}