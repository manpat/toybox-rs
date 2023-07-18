use crate::prelude::*;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FboName(pub u32);
impl FboName {
	pub const fn backbuffer() -> FboName { FboName(0) }
}


/// Fbo
impl super::Core {
	pub fn clear_framebuffer_color_buffer(&self, fbo: FboName, draw_buffer: i32, color: impl Into<common::Color>) {
		unsafe {
			self.gl.ClearNamedFramebufferfv(fbo.0, gl::COLOR, draw_buffer, color.into().to_array().as_ptr());
		}
	}
	pub fn clear_framebuffer_depth_stencil(&self, fbo: FboName, depth: f32, stencil: u8) {
		unsafe {
			self.gl.ClearNamedFramebufferfi(fbo.0, gl::DEPTH_STENCIL, 0, depth, stencil as i32);
		}
	}
}


impl super::ResourceName for FboName {
	const GL_IDENTIFIER: u32 = gl::FRAMEBUFFER;
	fn as_raw(&self) -> u32 { self.0 }
}