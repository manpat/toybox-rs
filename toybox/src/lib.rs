#![doc = include_str!("../README.md")]
// #![feature(let_chains)]

pub mod prelude;
pub use crate::prelude::*;

pub mod context;
pub use context::Context;

mod debug;


pub trait App {
	fn customise_debug_menu(&mut self, _: &mut egui::Ui) {}
	fn present(&mut self, _: &mut Context);
}


pub fn run<F, A>(title: &str, start_app: F) -> anyhow::Result<()>
	where A: App + 'static
		, F: FnOnce(&mut Context) -> anyhow::Result<A>
{
	run_with_settings(host::Settings::new(title), start_app)
}


pub fn run_with_settings<F, A>(settings: host::Settings<'_>, start_app: F) -> anyhow::Result<()>
	where A: App + 'static
		, F: FnOnce(&mut Context) -> anyhow::Result<A>
{
	let app_name = settings.initial_title;

	host::start(settings, move |host| {
		use anyhow::Context;
		
		let vfs = vfs::Vfs::new()
			.context("Initialising Vfs")?;

		log::info!("Resource Root Path: {}", vfs.resource_root().display());

		let winit::dpi::PhysicalSize{width, height} = host.window.inner_size().cast::<i32>();
		let backbuffer_size = Vec2i::new(width, height);

		let mut gfx = {
			let core = gfx::Core::new(host.gl.clone());
			gfx::System::new(core, &vfs)?
		};

		gfx.resize(backbuffer_size);

		let audio = audio::init()?;
		let input = input::System::new(host.window.clone());

		let egui = egui::Context::default();
		let egui_integration = egui_backend::Integration::new(egui.clone(), host.window.clone(), &mut gfx)?;

		let cfg = cfg::Config::for_app_name(app_name)?;

		let mut context = context::Context {
			gfx,
			audio,
			input,
			egui,
			cfg,
			vfs,

			egui_integration,
			egui_claiming_input_gate: Gate::new(),

			show_debug_menu: false,
			wants_quit: false,
		};

		let app = start_app(&mut context)?;

		Ok(HostedApp {
			context,
			debug_menu_state: debug::MenuState::default(),
			app,
		})
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

	fn draw(&mut self, event_loop: &host::ActiveEventLoop) {
		self.context.start_frame();

		debug::show_menu(&mut self.context, &mut self.app, &mut self.debug_menu_state);
		self.app.present(&mut self.context);

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