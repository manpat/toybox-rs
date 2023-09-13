pub use gl;
pub use winit;
pub use glutin;

use winit::{
	// event::{Event, WindowEvent, DeviceEvent, KeyboardInput, VirtualKeyCode},
	event_loop::EventLoop,
	window::WindowBuilder,
};

use glutin_winit::DisplayBuilder;

use glutin::prelude::*;
use glutin::config::{ConfigTemplateBuilder, Api};
use glutin::context::{GlProfile, ContextApi, Version, ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor};
use glutin::display::{GetGlDisplay};
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface, SwapInterval};

use raw_window_handle::HasRawWindowHandle;

use std::num::NonZeroU32;

pub mod prelude {
	pub use gl;
	pub use winit;
	pub use glutin;

	pub use glutin::prelude::*;
}


pub use winit::window::Window;

pub type Surface = glutin::surface::Surface<WindowSurface>;
pub type GlContext = glutin::context::PossiblyCurrentContext;


pub struct Host {
	pub event_loop: winit::event_loop::EventLoop<()>,

	pub window: Window,

	pub surface: Surface,
	pub gl_context: GlContext,

	pub gl_state: gl::Gl,
}


impl Host {
	pub fn create(title: &str) -> anyhow::Result<Host> {
		let event_loop = EventLoop::new();
		
		let config_template = ConfigTemplateBuilder::new()
			.with_api(Api::OPENGL)
			.with_stencil_size(8);

		let window_builder = WindowBuilder::new()
			.with_title(title);

		let context_builder = ContextAttributesBuilder::new()
			.with_debug(true)
			.with_profile(GlProfile::Core)
			.with_context_api(ContextApi::OpenGl(Some(Version::new(4, 5))));

		// Try to create our window and a config that describes a context we can create
		let (maybe_window, config) = DisplayBuilder::new()
			.with_window_builder(Some(window_builder))
			.build(&event_loop, config_template, |configs| {
				for config in configs {
					// We require an sRGB capable backbuffer
					if !config.srgb_capable() { continue }
					return config;
				}

				panic!("No suitable config");
			})
			.map_err(|e| anyhow::format_err!("Failed to create window: {e}"))?;


		let Some(window) = maybe_window else {
			anyhow::bail!("Failed to create a window")
		};

		let display = config.display();

		let gl_context = unsafe {
			let ctx_attributes = context_builder.build(Some(window.raw_window_handle()));
			display.create_context(&config, &ctx_attributes)?
		};


		let (width, height): (u32, u32) = window.inner_size().into();
		let attrs = SurfaceAttributesBuilder::<WindowSurface>::new()
			.with_srgb(Some(true))
			.build(
				window.raw_window_handle(),
				NonZeroU32::new(width).unwrap(),
				NonZeroU32::new(height).unwrap(),
			);

		let surface = unsafe { display.create_window_surface(&config, &attrs).unwrap() };


		let gl_context = gl_context.make_current(&surface).unwrap();
		surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())).unwrap();

		let gl_state = gl::Gl::load_with(|symbol| {
			let symbol = std::ffi::CString::new(symbol).unwrap();
			display.get_proc_address(symbol.as_c_str()).cast()
		});

		// Make sure sRGB handling is enabled by default.
		unsafe {
			gl_state.Enable(gl::FRAMEBUFFER_SRGB);
		}

		Ok(Host {
			event_loop,

			window,

			surface,
			gl_context,

			gl_state,
		})
	}

	pub fn install_default_error_handler(&self) {
		let gl = &self.gl_state;

		unsafe {
			gl.DebugMessageCallback(Some(default_gl_error_handler), std::ptr::null());
			gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);

			// Disable performance messages
			gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PERFORMANCE,
				gl::DONT_CARE,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable medium and low portability messages
			// Otherwise we get spammed about opengl es 3 portability which we don't care about.
			gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PORTABILITY,
				gl::DEBUG_SEVERITY_MEDIUM,
				0, std::ptr::null(),
				gl::FALSE
			);
			gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_PORTABILITY,
				gl::DEBUG_SEVERITY_LOW,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable notification messages
			gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DONT_CARE,
				gl::DEBUG_SEVERITY_NOTIFICATION,
				0, std::ptr::null(),
				gl::FALSE
			);

			// Disable marker messages
			gl.DebugMessageControl(
				gl::DONT_CARE,
				gl::DEBUG_TYPE_MARKER,
				gl::DONT_CARE,
				0, std::ptr::null(),
				gl::FALSE
			);
		}
	}
}


extern "system" fn default_gl_error_handler(source: u32, ty: u32, _id: u32, severity: u32,
	_length: i32, msg: *const i8, _ud: *mut std::ffi::c_void)
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
	eprintln!("Source:   {}", source);
	eprintln!("Severity: {}", severity_str);
	eprintln!("Type:     {}", ty_str);

	unsafe {
		let msg = std::ffi::CStr::from_ptr(msg as _).to_str().unwrap();
		eprintln!("Message: {}", msg);
	}

	match (severity, ty) {
		(_, gl::DEBUG_TYPE_PORTABILITY | gl::DEBUG_TYPE_PERFORMANCE | gl::DEBUG_TYPE_OTHER) => {}
		(gl::DEBUG_SEVERITY_HIGH | gl::DEBUG_SEVERITY_MEDIUM, _) => panic!("GL ERROR!"),
		_ => {}
	}
}