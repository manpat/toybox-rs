pub use gl;
pub use winit;
pub use glutin;

use std::rc::Rc;

use winit::{
	// event::{Event, WindowEvent, DeviceEvent, KeyboardInput, VirtualKeyCode},
	application::ApplicationHandler,
	event_loop::{EventLoop},
	window::{WindowId, WindowAttributes},
	dpi::{PhysicalPosition, PhysicalSize},
};

use glutin_winit::{DisplayBuilder, ApiPreference};

use glutin::prelude::*;
use glutin::config::{ConfigTemplateBuilder, Api};
use glutin::context::{GlProfile, ContextApi, Version, ContextAttributesBuilder, Robustness};
use glutin::display::{GetGlDisplay};
use glutin::surface::{WindowSurface, SwapInterval};

use raw_window_handle::HasWindowHandle;

use std::num::NonZeroU32;

pub mod prelude {
	pub use gl;
	pub use winit;
	pub use glutin;

	pub use glutin::prelude::*;
}

pub use winit::{
	event::{WindowEvent, StartCause, DeviceId, DeviceEvent},
	event_loop::{ActiveEventLoop},
	window::Window,
};

pub type Surface = glutin::surface::Surface<WindowSurface>;
pub type GlContext = glutin::context::PossiblyCurrentContext;



pub fn start<F, H>(settings: Settings<'_>, start_hostee: F) -> anyhow::Result<()>
	where F: FnOnce(&Host) -> anyhow::Result<Box<H>>
		, H: HostedApp + 'static
{
	let _span = tracing::info_span!("host start").entered();

	let event_loop = EventLoop::new()?;

	let window_attributes = Window::default_attributes()
		.with_title(settings.app_name)
		.with_transparent(settings.transparent)
		.with_decorations(!settings.no_decorations)
		.with_resizable(true)
		.with_visible(false);

	let gl_config_template = ConfigTemplateBuilder::new()
		.with_api(Api::OPENGL)
		.with_stencil_size(8) // TODO(pat.m): don't rely on default backbuffer
		.with_transparency(settings.transparent);

	let gl_context_attributes = ContextAttributesBuilder::new()
		.with_debug(true)
		.with_profile(GlProfile::Core)
		.with_robustness(Robustness::RobustLoseContextOnReset)
		.with_context_api(ContextApi::OpenGl(Some(Version::new(4, 6))));


	let bootstrap_state = BootstrapState {
		window_attributes,
		gl_config_template,
		gl_context_attributes,

		_span,
	};

	// NOTE: Box to avoid issues with stack sizes if hostee is too large.
	let mut app_host = Box::new(ApplicationHost::Bootstrap(bootstrap_state, start_hostee));
	event_loop.run_app(&mut app_host)?;

	Ok(())
}



#[derive(Default)]
enum ApplicationHost<F, H> {
	#[default]
	Empty,
	Bootstrap(BootstrapState, F),
	Hosting(Host, Box<H>),
}

impl<F, H> ApplicationHandler for ApplicationHost<F, H>
	where F: FnOnce(&Host) -> anyhow::Result<Box<H>>
		, H: HostedApp + 'static
{
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let ApplicationHost::Bootstrap(state, start_hostee) = std::mem::take(self) else { return };

		log::trace!("Bootstrapping ApplicationHost");

		let host = state.bootstrap(event_loop).expect("Failed to bootstrap application");

		// Enable vsync
		host.set_vsync(true);

		let mut hosted_app = start_hostee(&host)
			.expect("Failed to start hosted app");

		mark_tracy_frame();

		// Draw before making the window visisble
		hosted_app.draw(event_loop);

		host.window.pre_present_notify();
		host.swap();

		mark_tracy_frame();

		host.window.set_visible(true);
		log::info!("Window made visible");

		*self = ApplicationHost::Hosting(host, hosted_app);
	}

	// TODO(pat.m): is this even useful?
	fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
		if let ApplicationHost::Hosting(_, hosted_app) = self {
			hosted_app.new_events(event_loop, cause);
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
		let ApplicationHost::Hosting(host, hosted_app) = self else {
			return
		};

		if window_id != host.window.id() {
			// TODO(pat.m): manage child windows
			return
		}

		match event {
			WindowEvent::RedrawRequested => {
				hosted_app.draw(event_loop);

				host.window.pre_present_notify();
				host.swap();

				mark_tracy_frame();
			}

			event @ WindowEvent::Resized(physical_size) => {
				host.resize(physical_size.width, physical_size.height);
				hosted_app.window_event(event_loop, event);
			}

			event => {
				hosted_app.window_event(event_loop, event);
			}
		}
	}

	fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
		if let ApplicationHost::Hosting(_, hosted_app) = self {
			hosted_app.device_event(event_loop, device_id, event);
		}
	}

	fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
		if let ApplicationHost::Hosting(host, _) = self {
			host.window.request_redraw();
		}
	}

	fn exiting(&mut self, event_loop: &ActiveEventLoop) {
		if let ApplicationHost::Hosting(_, hosted_app) = self {
			hosted_app.shutdown(event_loop);
			return
		}
	}
}


pub trait HostedApp {
	fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {}

	fn window_event(&mut self, _: &ActiveEventLoop, _: WindowEvent) {}
	fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

	fn draw(&mut self, _: &ActiveEventLoop) {}

	fn shutdown(&mut self, _: &ActiveEventLoop) {}
}




pub struct Settings<'title> {
	pub app_name: &'title str,
	pub transparent: bool,
	pub no_decorations: bool,
}

impl<'title> Settings<'title> {
	pub fn new(app_name: &'title str) -> Self {
		Settings {
			app_name,
			transparent: false,
			no_decorations: false,
		}
	}

	pub fn transparent(mut self) -> Self {
		self.transparent = true;
		self
	}

	pub fn no_decorations(mut self) -> Self {
		self.no_decorations = true;
		self
	}
}




struct BootstrapState {
	window_attributes: WindowAttributes,

	gl_config_template: ConfigTemplateBuilder,
	gl_context_attributes: ContextAttributesBuilder,

	_span: tracing::span::EnteredSpan,
}

impl BootstrapState {
	fn bootstrap(mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<Host> {
		// Try to fit window to monitor
		if let Some(monitor) = event_loop.primary_monitor()
			.or_else(|| event_loop.available_monitors().next())
		{
			let PhysicalPosition{x, y} = monitor.position();
			let PhysicalSize{width, height} = monitor.size();

			self.window_attributes = self.window_attributes.with_inner_size(PhysicalSize{
				width: width.checked_sub(100).unwrap_or(width),
				height: height.checked_sub(100).unwrap_or(height),
			});

			self.window_attributes = self.window_attributes.with_position(PhysicalPosition{
				x: x + 50,
				y: y + 30, // Fudged for window decorations lol
			});
		} else {
			log::warn!("Couldn't get primary monitor - using default window size");
		}

		// Try to create our window and a config that describes a context we can create
		let _span = tracing::info_span!("host build display").entered();

		let (maybe_window, gl_config) = DisplayBuilder::new()
			.with_window_attributes(Some(self.window_attributes.clone()))
			.with_preference(ApiPreference::PreferEgl)
			.build(event_loop, self.gl_config_template, |configs| {
				for config in configs {
					// We require an sRGB capable backbuffer
					if !config.srgb_capable() { continue }
					return config;
				}

				panic!("No suitable config");
			})
			.map_err(|e| anyhow::format_err!("Failed to find suitable surface config: {e}"))?;

		_span.exit();

		log::info!("Display built with config: {gl_config:?}");

		let maybe_raw_window_handle = maybe_window
			.as_ref()
			.and_then(|window| window.window_handle().ok())
			.map(|handle| handle.as_raw());

		let _span = tracing::info_span!("host create opengl context").entered();

		let gl_context_attributes = self.gl_context_attributes.build(maybe_raw_window_handle);
		let gl_display = gl_config.display();

		// Create our context
		let non_current_gl_context = unsafe {
			gl_display.create_context(&gl_config, &gl_context_attributes)?
		};

		_span.exit();

		log::info!("Context created with {gl_context_attributes:?}");

		// Create our window for real if not already
		let window = match maybe_window {
			Some(window) => window,
			None => {
				let _span = tracing::info_span!("host finalize window").entered();
				glutin_winit::finalize_window(event_loop, self.window_attributes.clone(), &gl_config)?
			}
		};

		let window = Rc::new(window);

		// Create a surface
		let _span = tracing::info_span!("host build surface").entered();
		let (width, height): (u32, u32) = window.inner_size().into();
		let surface_attributes = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new()
			.with_srgb(Some(true))
			.build(
				window.window_handle()?.as_raw(),
				NonZeroU32::new(width).unwrap(),
				NonZeroU32::new(height).unwrap(),
			);

		let gl_surface = unsafe {
			gl_display.create_window_surface(&gl_config, &surface_attributes)?
		};

		_span.exit();

		// Finally make our context current
		let gl_context = non_current_gl_context.make_current(&gl_surface)?;

		let gl = tracing::info_span!("host load opengl").in_scope(|| gl::Gl::load_with(|symbol| {
			let symbol = std::ffi::CString::new(symbol).unwrap();
			gl_display.get_proc_address(symbol.as_c_str()).cast()
		}));

		Ok(Host {
			context: gl_context,
			gl,

			window,
			surface: gl_surface,

			config: gl_config,
			window_attributes: self.window_attributes,
		})
	}
}


pub struct Host {
	pub context: glutin::context::PossiblyCurrentContext,
	pub gl: gl::Gl,

	pub config: glutin::config::Config,
	pub window_attributes: WindowAttributes,

	pub window: Rc<Window>,
	pub surface: glutin::surface::Surface<WindowSurface>,
}

impl Host {
	pub fn set_vsync(&self, enabled: bool) {
		let interval = match enabled {
			false => SwapInterval::DontWait,
			true => SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
		};

		if let Err(error) = self.surface.set_swap_interval(&self.context, interval) {
			log::warn!("Failed to set swap interval: {error}");
		}
	}

	pub fn swap(&self) {
		if let Err(error) = self.surface.swap_buffers(&self.context) {
			// TODO(pat.m): possibly try to recreate surface if lost
			panic!("Failed to swap: {error}");
		}
	}

	pub fn resize(&self, width: u32, height: u32) {
		if let Some((width, height)) = NonZeroU32::new(width).zip(NonZeroU32::new(height)) {
			self.surface.resize(&self.context, width, height);
		}
	}
}

fn init_logging() {
	let mut log_builder = env_logger::builder();
	log_builder.parse_default_env();
	log_builder.format_timestamp_millis();
	log_builder.format_indent(None);

	if cfg!(debug_assertions) {
		log_builder.filter_level(log::LevelFilter::Debug);
	}

	log_builder.init();

	log::info!("Logger initialized");
}

#[cfg(feature="tracy")]
fn init_tracy() {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(tracing_tracy::TracyLayer::default());

    tracing::subscriber::set_global_default(subscriber)
    	.expect("set up the subscriber");

	log::info!("Tracy initialized");
}

fn mark_tracy_frame() {
	#[cfg(feature="tracy")]
	if let Some(client) = tracy_client::Client::running() {
		client.frame_mark();
	}
}


/// Initialise logging and profiling if configured. Must be called before anything else.
pub fn init_environment() {
	init_logging();

	#[cfg(feature="tracy")]
	init_tracy();
}