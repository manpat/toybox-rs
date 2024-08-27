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

	current_viewport_size: Cell<Vec2i>,

	global_vao_name: u32,

	buffer_info: RefCell<HashMap<BufferName, BufferInfo>>,
	image_info: RefCell<HashMap<ImageName, ImageInfoInternal>>,
	framebuffer_info: RefCell<HashMap<FramebufferName, FramebufferInfo>>,

	backbuffer_size: Vec2i,
}

impl Core {
	pub fn new(gl: gl::Gl) -> Core {
		let capabilities = Capabilities::from(&gl);
		let global_vao_name = Self::create_and_bind_global_vao(&gl);

		let get_string = |name| unsafe {
			std::ffi::CStr::from_ptr(gl.GetString(name).cast())
				.to_string_lossy()
		};

		log::info!("OpenGL Vendor: {}", get_string(gl::VENDOR));
		log::info!("OpenGL Renderer: {}", get_string(gl::RENDERER));
		log::info!("OpenGL Version: {}", get_string(gl::VERSION));
		log::info!("GLSL Version: {}", get_string(gl::SHADING_LANGUAGE_VERSION));

		log::info!("OpenGL {capabilities:#?}");

		Core {
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

			current_viewport_size: Cell::new(Vec2i::zero()),

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

	pub fn set_debug_label<N>(&self, name: N, label: impl AsRef<str>)
		where N: ResourceName
	{
		let label = label.as_ref();

		unsafe {
			self.gl.ObjectLabel(N::GL_IDENTIFIER, name.as_raw(), label.len() as i32, label.as_ptr() as *const _);
		}
	}

	pub fn set_debugging_enabled(&self, enabled: bool) {
		unsafe {
			match enabled {
				true => self.gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS),
				false => self.gl.Disable(gl::DEBUG_OUTPUT_SYNCHRONOUS),
			}
		}
	}

	pub fn register_debug_hook(&self) {
		unsafe {
			self.gl.DebugMessageCallback(Some(default_gl_error_handler), std::ptr::null());

			// Disable performance messages
			self.gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PERFORMANCE,
				gl::DONT_CARE,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable medium and low portability messages
			// Otherwise we get spammed about opengl es 3 portability which we don't care about.
			self.gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PORTABILITY,
				gl::DEBUG_SEVERITY_MEDIUM,
				0, std::ptr::null(),
				gl::FALSE
			);
			self.gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PORTABILITY,
				gl::DEBUG_SEVERITY_LOW,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable notification messages
			self.gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DONT_CARE,
				gl::DEBUG_SEVERITY_NOTIFICATION,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable marker messages
			self.gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_MARKER,
				gl::DONT_CARE,
				0, std::ptr::null(),
				gl::FALSE
			);
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



extern "system" fn default_gl_error_handler(source: u32, ty: u32, msg_id: u32, severity: u32,
	length: i32, msg: *const i8, _ud: *mut std::ffi::c_void)
{
	let severity_str = match severity {
		gl::DEBUG_SEVERITY_HIGH => "high",
		gl::DEBUG_SEVERITY_MEDIUM => "medium",
		gl::DEBUG_SEVERITY_LOW => "low",
		gl::DEBUG_SEVERITY_NOTIFICATION => return,
		_ => panic!("Unknown severity {}", severity),
	};

	let ty_str = match ty {
		gl::DEBUG_TYPE_ERROR => "error",
		gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "deprecated behaviour",
		gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "undefined behaviour",
		gl::DEBUG_TYPE_PORTABILITY => "portability",
		gl::DEBUG_TYPE_PERFORMANCE => "performance",
		gl::DEBUG_TYPE_OTHER => "other",
		_ => panic!("Unknown type {}", ty),
	};

	let source = match source {
		gl::DEBUG_SOURCE_API => "api",
		gl::DEBUG_SOURCE_WINDOW_SYSTEM => "window system",
		gl::DEBUG_SOURCE_SHADER_COMPILER => "shader compiler",
		gl::DEBUG_SOURCE_THIRD_PARTY => "third party",
		gl::DEBUG_SOURCE_APPLICATION => "application",
		gl::DEBUG_SOURCE_OTHER => "other",
		_ => panic!("Unknown source {}", source),
	};

	eprintln!("GL ERROR!");
	eprintln!("Source:   {source}");
	eprintln!("Severity: {severity_str}");
	eprintln!("Type:     {ty_str}");
	eprintln!("Id:       {msg_id}");

	unsafe {
		let msg_slice = std::slice::from_raw_parts(msg.cast(), length as usize);
		let msg_utf8 = String::from_utf8_lossy(msg_slice);
		eprintln!("Message: {}", msg_utf8);
	}

	match (severity, ty) {
		(_, gl::DEBUG_TYPE_PORTABILITY | gl::DEBUG_TYPE_PERFORMANCE | gl::DEBUG_TYPE_OTHER) => {}
		(gl::DEBUG_SEVERITY_HIGH | gl::DEBUG_SEVERITY_MEDIUM, _) => panic!("GL ERROR!"),
		_ => {}
	}
}