#![doc = include_str!("../README.md")]

pub mod prelude;
pub use crate::prelude::*;

pub mod context;
pub use context::Context;

mod debug;


pub trait App {
	fn customise_debug_menu(&mut self, _: &mut Context, _: &mut egui::Ui) {}
	fn present(&mut self, _: &mut Context);
}


pub fn run<F, A>(app_name: &str, start_app: F) -> anyhow::Result<()>
	where A: App + 'static
		, F: FnOnce(&mut Context) -> anyhow::Result<A>
{
	run_with_settings(host::Settings::new(app_name), start_app)
}


pub fn run_with_settings<F, A>(settings: host::Settings<'_>, start_app: F) -> anyhow::Result<()>
	where A: App + 'static
		, F: FnOnce(&mut Context) -> anyhow::Result<A>
{
	host::init_environment();

	let _span = tracing::info_span!("toybox early start").entered();

	let vfs = vfs::Vfs::new(settings.app_name)
		.context("Initialising Vfs")?;

	let cfg = cfg::Config::from_vfs(&vfs)?;
	let audio = audio::System::init();

	_span.exit();

	host::start(settings, move |host| {
		let _span = tracing::info_span!("toybox start").entered();

		let winit::dpi::PhysicalSize{width, height} = host.window.inner_size().cast::<i32>();
		let backbuffer_size = Vec2i::new(width, height);

		let mut gfx = tracing::info_span!("init gfx").in_scope(|| {
			let core = gfx::Core::new(host.gl.clone());
			gfx::System::new(core)
		})?;

		gfx.resize(backbuffer_size);

		let bus = bus::MessageBus::new();
		let input = input::System::new(host.window.clone());

		let egui = egui::Context::default();
		let egui_integration = egui_backend::Integration::new(egui.clone(), host.window.clone(), &mut gfx)?;

		let mut context = context::Context {
			gfx,
			audio,
			input,
			egui,
			cfg,
			vfs,
			bus,

			egui_integration,
			egui_claiming_input_gate: Gate::new(),

			show_debug_menu: false,
			wants_quit: false,
		};

		// Required since we now call this at the end of frames rather than the beginning.
		context.prepare_frame();

		let app = tracing::info_span!("app start").in_scope(|| start_app(&mut context))?;

		Ok(Box::new(HostedApp {
			context,
			debug_menu_state: debug::MenuState::default(),
			app,
		}))
	})
}





struct HostedApp<A: App> {
	context: context::Context,
	debug_menu_state: debug::MenuState,
	app: A,
}


impl<A: App> host::HostedApp for HostedApp<A> {
	fn window_event(&mut self, _: &host::ActiveEventLoop, event: host::WindowEvent) {
		if self.context.egui_integration.on_event(&event) {
			self.context.input.tracker.track_focus_lost();
			return
		}

		match event {
			host::WindowEvent::CloseRequested => {
				self.context.wants_quit = true;
			}

			host::WindowEvent::Resized(physical_size) => {
				let new_size = Vec2i::new(physical_size.width as i32, physical_size.height as i32);
				self.context.notify_resized(new_size);
				// self.app.resize(new_size);
			}

			event => {
				self.context.input.on_window_event(&event);
			}
		}
	}

	fn device_event(&mut self, _: &host::ActiveEventLoop, _: host::DeviceId, event: host::DeviceEvent) {
		self.context.input.on_device_event(&event);
	}

	#[instrument(skip_all, name="toybox draw")]
	fn draw(&mut self, event_loop: &host::ActiveEventLoop) {
		self.context.start_frame();

		debug::show_menu(&mut self.context, &mut self.app, &mut self.debug_menu_state);

		tracing::info_span!("app present").in_scope(|| {
			self.app.present(&mut self.context);
		});

		self.context.finalize_frame();

		if self.context.wants_quit {
			event_loop.exit();
		}

		self.context.prepare_frame();
	}

	fn shutdown(&mut self, _: &host::ActiveEventLoop) {
		self.context.shutdown();
	}
}