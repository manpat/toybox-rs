pub mod raw {
	include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub mod vao;
pub mod buffer;
pub mod texture;
pub mod vertex;
pub mod shader;

pub use self::vao::*;
pub use self::buffer::*;
pub use self::texture::*;
pub use self::vertex::*;
pub use self::shader::*;


pub struct Context {
	_sdl_ctx: sdl2::video::GLContext,
	shader_manager: ShaderManager,
}


pub enum DrawMode {
	Points,
	Lines,
	Triangles,
}



impl Context {
	pub fn new(sdl_ctx: sdl2::video::GLContext) -> Self {
		unsafe {
			raw::DebugMessageCallback(Some(gl_message_callback), std::ptr::null());
			raw::Enable(raw::DEBUG_OUTPUT_SYNCHRONOUS);
			raw::Enable(raw::PROGRAM_POINT_SIZE);

			raw::Enable(raw::DEPTH_TEST);
			// raw::Enable(raw::BLEND);
			// raw::BlendFunc(raw::DST_COLOR, raw::ZERO);
			// raw::BlendEquation(raw::FUNC_ADD);

			// Disable performance messages
			raw::DebugMessageControl(
				raw::DONT_CARE,
				raw::DEBUG_TYPE_PERFORMANCE,
				raw::DONT_CARE,
				0, std::ptr::null(),
				0 // false
			);

			// Disable notification messages
			raw::DebugMessageControl(
				raw::DONT_CARE,
				raw::DONT_CARE,
				raw::DEBUG_SEVERITY_NOTIFICATION,
				0, std::ptr::null(),
				0 // false
			);
		}

		Context {
			_sdl_ctx: sdl_ctx,
			shader_manager: ShaderManager::new(),
		}
	}


	pub fn set_wireframe(&self, wireframe_enabled: bool) {
		let mode = match wireframe_enabled {
			false => raw::FILL,
			true => raw::LINE,
		};

		unsafe {
			raw::PolygonMode(raw::FRONT_AND_BACK, mode);
		}
	}

	pub fn new_untyped_buffer(&self) -> UntypedBuffer {
		unsafe {
			let mut buf = 0;
			raw::CreateBuffers(1, &mut buf);
			UntypedBuffer(buf)
		}
	}

	pub fn new_buffer<T: Copy>(&self) -> Buffer<T> {
		self.new_untyped_buffer().into_typed()
	}

	pub fn new_texture(&self, width: u32, height: u32, format: u32) -> Texture {
		unsafe {
			let mut tex = 0;
			raw::CreateTextures(raw::TEXTURE_2D, 1, &mut tex);
			raw::TextureStorage2D(tex, 1, format, width as i32, height as i32);
			raw::TextureParameteri(tex, raw::TEXTURE_MIN_FILTER, raw::LINEAR as _);
			Texture(tex)
		}
	}

	pub fn new_vao(&self) -> Vao {
		unsafe {
			let mut vao = 0;
			raw::CreateVertexArrays(1, &mut vao);
			Vao::new(vao)
		}
	}

	pub fn bind_uniform_buffer(&self, binding: u32, buffer: impl Into<UntypedBuffer>) {
		let buffer = buffer.into();
		unsafe {
			raw::BindBufferBase(raw::UNIFORM_BUFFER, binding, buffer.0);
		}
	}

	pub fn bind_shader_storage_buffer(&self, binding: u32, buffer: impl Into<UntypedBuffer>) {
		let buffer = buffer.into();
		unsafe {
			raw::BindBufferBase(raw::SHADER_STORAGE_BUFFER, binding, buffer.0);
		}
	}

	pub fn bind_image_rw(&self, binding: u32, texture: Texture, format: u32) {
		unsafe {
			let (level, layered, layer) = (0, 0, 0);
			raw::BindImageTexture(binding, texture.0, level, layered, layer, raw::READ_WRITE, format);
		}
	}

	pub fn bind_texture(&self, binding: u32, texture: Texture) {
		unsafe {
			raw::BindTextureUnit(binding, texture.0);
		}
	}

	pub fn bind_vao(&self, vao: Vao) {
		unsafe {
			raw::BindVertexArray(vao.handle);
		}
	}


	pub fn add_shader_import(&mut self, name: impl Into<String>, src: impl Into<String>) {
		self.shader_manager.add_import(name, src)
	}

	pub fn new_shader(&self, shaders: &[(u32, &str)]) -> Result<Shader, shader::CompilationError> {
		self.shader_manager.get_shader(shaders)
	}

	pub fn bind_shader(&self, shader: Shader) {
		unsafe {
			raw::UseProgram(shader.0);
		}
	}

	pub fn draw_indexed(&self, draw_mode: DrawMode, num_elements: u32) {
		unsafe {
			raw::DrawElements(draw_mode.into_gl(), num_elements as i32, raw::UNSIGNED_SHORT, std::ptr::null());
		}
	}

	pub fn draw_arrays(&self, draw_mode: DrawMode, num_vertices: u32) {
		unsafe {
			raw::DrawArrays(draw_mode.into_gl(), 0, num_vertices as i32);
		}
	}

	pub fn dispatch_compute(&self, x: u32, y: u32, z: u32) {
		unsafe {
			raw::DispatchCompute(x, y, z);
		}
	}
}



impl DrawMode {
	fn into_gl(self) -> u32 {
		match self {
			DrawMode::Points => raw::POINTS,
			DrawMode::Lines => raw::LINES,
			DrawMode::Triangles => raw::TRIANGLES,
		}
	}
}



extern "system" fn gl_message_callback(source: u32, ty: u32, _id: u32, severity: u32,
	_length: i32, msg: *const i8, _ud: *mut std::ffi::c_void)
{
	let severity = match severity {
		raw::DEBUG_SEVERITY_LOW => "low",
		raw::DEBUG_SEVERITY_MEDIUM => "medium",
		raw::DEBUG_SEVERITY_HIGH => "high",
		raw::DEBUG_SEVERITY_NOTIFICATION => "notification",
		_ => panic!("Unknown severity {}", severity),
	};

	let ty = match ty {
		raw::DEBUG_TYPE_ERROR => "error",
		raw::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "deprecated behaviour",
		raw::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "undefined behaviour",
		raw::DEBUG_TYPE_PORTABILITY => "portability",
		raw::DEBUG_TYPE_PERFORMANCE => "performance",
		raw::DEBUG_TYPE_OTHER => "other",
		_ => panic!("Unknown type {}", ty),
	};

	let source = match source {
		raw::DEBUG_SOURCE_API => "api",
		raw::DEBUG_SOURCE_WINDOW_SYSTEM => "window system",
		raw::DEBUG_SOURCE_SHADER_COMPILER => "shader compiler",
		raw::DEBUG_SOURCE_THIRD_PARTY => "third party",
		raw::DEBUG_SOURCE_APPLICATION => "application",
		raw::DEBUG_SOURCE_OTHER => "other",
		_ => panic!("Unknown source {}", source),
	};

	eprintln!("GL ERROR!");
	eprintln!("Source:   {}", source);
	eprintln!("Severity: {}", severity);
	eprintln!("Type:     {}", ty);

	unsafe {
		let msg = std::ffi::CStr::from_ptr(msg as _).to_str().unwrap();
		eprintln!("Message: {}", msg);
	}

	panic!("GL ERROR!");
}