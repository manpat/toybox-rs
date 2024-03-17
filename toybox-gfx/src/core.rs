use crate::prelude::*;

use toybox_host as host;
use host::prelude::*;

pub mod capabilities;
pub mod fbo;
pub mod vao;
mod buffer;
pub mod barrier;
pub mod sampler;
mod image;
pub mod shader;
pub mod shader_pipeline;
pub mod global_state;

pub use capabilities::Capabilities;
pub use fbo::*;
pub use buffer::*;
pub use sampler::{SamplerName, AddressingMode, FilterMode};
pub use self::image::*;
pub use shader::{ShaderName, ShaderType};
pub use shader_pipeline::{ShaderPipelineName};
pub use global_state::*;

use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;


pub struct Core {
	surface: host::Surface,
	gl_context: host::GlContext,
	pub gl: gl::Gl,
	capabilities: Capabilities,

	barrier_tracker: RefCell<barrier::BarrierTracker>,

	num_active_clip_planes: Cell<u32>,
	bound_index_buffer: Cell<Option<BufferName>>,
	bound_shader_pipeline: Cell<ShaderPipelineName>,
	bound_framebuffer: Cell<Option<FramebufferName>>,
	// TODO(pat.m): bound samplers and texture units

	current_blend_mode: Cell<Option<BlendMode>>,
	depth_test_enabled: Cell<bool>,
	depth_write_enabled: Cell<bool>,

	global_vao_name: u32,

	buffer_info: RefCell<HashMap<BufferName, BufferInfo>>,
	image_info: RefCell<HashMap<ImageName, ImageInfo>>,
	framebuffer_info: RefCell<HashMap<FramebufferName, FramebufferInfo>>,

	backbuffer_size: Vec2i,
}

impl Core {
	pub fn new(surface: host::Surface, gl_context: host::GlContext, gl: gl::Gl)
		-> Core
	{
		let capabilities = Capabilities::from(&gl);
		let global_vao_name = Self::create_and_bind_global_vao(&gl);

		Core {
			surface,
			gl_context,
			gl,
			capabilities,

			barrier_tracker: RefCell::new(barrier::BarrierTracker::new()),

			num_active_clip_planes: Cell::new(0),
			bound_index_buffer: Cell::new(None),
			bound_framebuffer: Cell::new(None),
			bound_shader_pipeline: Cell::new(ShaderPipelineName(0)),

			current_blend_mode: Cell::new(None),
			depth_test_enabled: Cell::new(true),
			depth_write_enabled: Cell::new(true),

			global_vao_name,

			buffer_info: RefCell::new(HashMap::new()),
			image_info: RefCell::new(HashMap::new()),
			framebuffer_info: RefCell::new(HashMap::new()),

			backbuffer_size: Vec2i::zero(),
		}
	}

	pub fn capabilities(&self) -> &Capabilities {
		&self.capabilities
	}

	pub fn barrier_tracker(&self) -> RefMut<'_, barrier::BarrierTracker> {
		self.barrier_tracker.borrow_mut()
	}

	pub fn swap(&mut self) {
		// HACK: For some reason capturing the app with discord (and I suspect other window capture apps)
		// causes swap_buffers to emit GL_INVALID_ENUM on my machine, which panics ofc.
		// So for now, we are just not emitting errors pls.
		unsafe {
			self.gl.Disable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
		}

		if let Err(error) = self.surface.swap_buffers(&self.gl_context) {
			println!("Failed to swap!\n{error}");
		}

		unsafe {
			self.gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
		}
	}

	pub fn backbuffer_size(&self) -> Vec2i {
		self.backbuffer_size
	}

	pub(crate) fn set_backbuffer_size(&mut self, new_size: Vec2i) {
		self.backbuffer_size = new_size;
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

	pub fn set_debug_label<N>(&self, name: N, label: &str)
		where N: ResourceName
	{
		unsafe {
			self.gl.ObjectLabel(N::GL_IDENTIFIER, name.as_raw(), label.len() as i32, label.as_ptr() as *const _);
		}
	}
}


/// Features
impl Core {
	pub fn set_user_clip_planes(&self, new_count: u32) {
		assert!(new_count <= self.capabilities.max_user_clip_planes as u32, "GL_MAX_CLIP_DISTANCES exceeded");

		let current_count = self.num_active_clip_planes.get();

		if new_count > current_count {
			for i in current_count..new_count {
				unsafe {
					self.gl.Enable(gl::CLIP_DISTANCE0 + i);
				}
			}
		}

		if new_count < current_count {
			for i in new_count..current_count {
				unsafe {
					self.gl.Disable(gl::CLIP_DISTANCE0 + i);
				}
			}
		}

		self.num_active_clip_planes.set(new_count);
	}
}


impl Drop for Core {
	fn drop(&mut self) {
		self.destroy_global_vao();
	}
}



pub trait ResourceName {
	const GL_IDENTIFIER: u32;
	fn as_raw(&self) -> u32;
}

impl<T> ResourceName for Option<T>
	where T: ResourceName
{
	const GL_IDENTIFIER: u32 = T::GL_IDENTIFIER;
	fn as_raw(&self) -> u32 {
		if let Some(inner) = self {
			inner.as_raw()
		} else {
			0
		}
	}
}