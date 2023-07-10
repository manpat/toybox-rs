pub use gl;
pub use winit;
pub use glutin;

use winit::{
	// event::{Event, WindowEvent, DeviceEvent, KeyboardInput, VirtualKeyCode},
	event_loop::EventLoop,
	window::WindowBuilder,
};

use glutin_winit::DisplayBuilder;

use glutin::config::{ConfigTemplateBuilder, Api};
use glutin::context::{GlProfile, ContextApi, Version, ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor};
use glutin::display::{GlDisplay, GetGlDisplay};
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface, SwapInterval};
use glutin::prelude::GlSurface;

use raw_window_handle::HasRawWindowHandle;

use std::num::NonZeroU32;

// use common::math::Vec2i;




// pub fn run<F, M>(start_main_loop: F) -> anyhow::Result<()> {

	// let config_template = ConfigTemplateBuilder::new()
	// 	.with_api(Api::OPENGL)
	// 	.with_stencil_size(8);

	// let window_builder = WindowBuilder::new();

	// let (window, config) = DisplayBuilder::new()
	// 	.with_window_builder(Some(window_builder))
	// 	.build(&event_loop, config_template, |configs| {
	// 		use glutin::config::GlConfig;

	// 		for config in configs {
	// 			if !config.srgb_capable() { continue }
	// 			return config;
	// 		}

	// 		panic!("No suitable config");
	// 	})
	// 	.unwrap();

	// let window = window.unwrap();

	// let display = config.display();

	// let ctx_attributes = ContextAttributesBuilder::new()
	// 	.with_debug(true)
	// 	.with_profile(GlProfile::Core)
	// 	.with_context_api(ContextApi::OpenGl(Some(Version::new(4, 5))))
	// 	.build(Some(window.raw_window_handle()));

	// let context = unsafe {
	// 	display.create_context(&config, &ctx_attributes).unwrap()
	// };


	// let (width, height): (u32, u32) = window.inner_size().into();
	// let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().with_srgb(Some(true)).build(
	// 	window.raw_window_handle(),
	// 	NonZeroU32::new(width).unwrap(),
	// 	NonZeroU32::new(height).unwrap(),
	// );

	// let surface = unsafe { display.create_window_surface(&config, &attrs).unwrap() };


	// let context = context.make_current(&surface).unwrap();
	// surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())).unwrap();

	// let gl = gl::Gl::load_with(|symbol| {
	// 	let symbol = std::ffi::CString::new(symbol).unwrap();
	// 	display.get_proc_address(symbol.as_c_str()).cast()
	// });


	// Set up srgb backbuffer
	// unsafe {
	//     gl.Enable(gl::FRAMEBUFFER_SRGB);
	// }


	// Set up debug callbacks
	// unsafe {
	//     gl.DebugMessageCallback(Some(gl_message_callback), std::ptr::null());
	//     gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);

	//     // Disable performance messages
	//     // gl.DebugMessageControl(
	//     //  gl::DONT_CARE,
	//     //  gl::DEBUG_TYPE_PERFORMANCE,
	//     //  gl::DONT_CARE,
	//     //  0, std::ptr::null(),
	//     //  0 // false
	//     // );

	//     // Disable notification messages
	//     gl.DebugMessageControl(
	//         gl::DONT_CARE,
	//         gl::DONT_CARE,
	//         gl::DEBUG_SEVERITY_NOTIFICATION,
	//         0, std::ptr::null(),
	//         0 // false
	//     );
	// }
// }



pub struct Host {
	pub event_loop: winit::event_loop::EventLoop<()>,

	pub window: winit::window::Window,

	pub surface: glutin::surface::Surface<WindowSurface>,
	pub gl_context: glutin::context::PossiblyCurrentContext,

	pub gl_state: gl::Gl,
}


impl Host {
	pub fn create() -> anyhow::Result<Host> {
		let event_loop = EventLoop::new();
		
		let config_template = ConfigTemplateBuilder::new()
			.with_api(Api::OPENGL)
			.with_stencil_size(8);

		let window_builder = WindowBuilder::new()
			.with_title("AAAAAAA");

		let context_builder = ContextAttributesBuilder::new()
			.with_debug(true)
			.with_profile(GlProfile::Core)
			.with_context_api(ContextApi::OpenGl(Some(Version::new(4, 5))));

		// Try to create our window and a config that describes a context we can create
		let (maybe_window, config) = DisplayBuilder::new()
			.with_window_builder(Some(window_builder))
			.build(&event_loop, config_template, |configs| {
				use glutin::config::GlConfig;

				for config in configs {
					if !config.srgb_capable() { continue }
					return config;
				}

				panic!("No suitable config");
			})
			.map_err(|e| anyhow::format_err!("Failed to create window: {e}"))?;


		let Some(window) = maybe_window else {
			anyhow::bail!("Failed to create a window")
		};

		let ctx_attributes = context_builder.build(Some(window.raw_window_handle()));
		let display = config.display();

		let gl_context = unsafe {
			display.create_context(&config, &ctx_attributes).unwrap()
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

		Ok(Host {
			event_loop,

			window,

			surface,
			gl_context,

			gl_state,
		})
	}
}