use crate::prelude::*;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FboHandle(pub u32);
impl FboHandle {
	pub const fn backbuffer() -> FboHandle { FboHandle(0) }
}


/// Fbo
impl super::Core {
	pub fn clear_framebuffer_color_buffer(&self, fbo: FboHandle, draw_buffer: i32, color: impl Into<common::Color>) {
		unsafe {
			self.gl.ClearNamedFramebufferfv(fbo.0, gl::COLOR, draw_buffer, color.into().to_array().as_ptr());
		}
	}
	pub fn clear_framebuffer_depth_stencil(&self, fbo: FboHandle, depth: f32, stencil: u8) {
		unsafe {
			self.gl.ClearNamedFramebufferfi(fbo.0, gl::DEPTH_STENCIL, 0, depth, stencil as i32);
		}
	}
}