use crate::prelude::*;
use crate::core::ImageName;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FramebufferName(pub u32);
impl FramebufferName {
	pub const fn backbuffer() -> FramebufferName { FramebufferName(0) }
}


impl super::ResourceName for FramebufferName {
	const GL_IDENTIFIER: u32 = gl::FRAMEBUFFER;
	fn as_raw(&self) -> u32 { self.0 }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FramebufferAttachment {
	Color(u32),
	Depth,
	Stencil,
	DepthStencil,
}


/// Framebuffer
impl super::Core {
	// TODO(pat.m): these are real awkward to use and not really useful outside of clearing the default framebuffer
	pub fn clear_framebuffer_color_buffer(&self, fbo: FramebufferName, draw_buffer: i32, color: impl Into<common::Color>) {
		unsafe {
			self.gl.ClearNamedFramebufferfv(fbo.as_raw(), gl::COLOR, draw_buffer, color.into().to_array().as_ptr());
		}
	}

	pub fn clear_framebuffer_depth_stencil(&self, fbo: FramebufferName, depth: f32, stencil: u8) {
		unsafe {
			self.gl.ClearNamedFramebufferfi(fbo.as_raw(), gl::DEPTH_STENCIL, 0, depth, stencil as i32);
		}
	}

	pub fn create_framebuffer(&self) -> FramebufferName {
		FramebufferName(unsafe {
			let mut name = 0;
			self.gl.CreateFramebuffers(1, &mut name);
			name
		})
	}

	pub fn destroy_framebuffer(&self, name: FramebufferName) {
		unsafe {
			self.gl.DeleteFramebuffers(1, &name.as_raw())
		}
	}

	pub fn bind_framebuffer(&self, name: impl Into<Option<FramebufferName>>) {
		let name = name.into();

		if self.bound_framebuffer.get() != name {
			unsafe {
				self.gl.BindFramebuffer(gl::FRAMEBUFFER, name.unwrap_or(FramebufferName(0)).as_raw());
			}

			self.bound_framebuffer.set(name);
		}
	}

	pub fn set_framebuffer_attachment(&self, fbo: FramebufferName, attachment: FramebufferAttachment, image: ImageName) {
		let attachment = match attachment {
			FramebufferAttachment::Color(index) => gl::COLOR_ATTACHMENT0 + index,
			FramebufferAttachment::Depth => gl::DEPTH_ATTACHMENT,
			FramebufferAttachment::Stencil => gl::STENCIL_ATTACHMENT,
			FramebufferAttachment::DepthStencil => gl::DEPTH_STENCIL_ATTACHMENT,
		};

		let level = 0;

		unsafe {
			self.gl.NamedFramebufferTexture(fbo.as_raw(), attachment, image.as_raw(), level);
		}
	}
}