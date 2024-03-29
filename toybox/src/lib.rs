#![doc = include_str!("../README.md")]
#![feature(let_chains)]

pub mod prelude;
pub use crate::prelude::*;

pub mod context;
pub use context::Context;

mod debug;
mod resources;

use host::Host;


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

	let host = Host::create(settings)?;
	host.install_default_error_handler();

	let Host{ event_loop, gl_state: gl, surface, gl_context, window, .. } = host;

	let window = std::rc::Rc::new(window);

	use anyhow::Context;
	let resource_root_path = resources::find_resource_folder()
		.context("Can't find resource directory")?;

	let mut gfx = {
		let core = gfx::Core::new(surface, gl_context, gl);
		gfx::System::new(core, &resource_root_path)?
	};

	let audio = audio::init()?;
	let input = input::System::new(window.clone());

	let egui = egui::Context::default();
	let egui_integration = egui_backend::Integration::new(egui.clone(), window.clone(), &mut gfx)?;

	let cfg = cfg::Config::for_app_name(app_name)?;

	let mut context = context::Context {
		gfx,
		audio,
		input,
		egui,
		cfg,

		egui_integration,
		egui_claiming_input_gate: Gate::new(),

		resource_root_path,

		show_debug_menu: false,
		wants_quit: false,
	};

	let mut app = start_app(&mut context)?;

	let mut debug_menu_state = debug::MenuState::default();

	event_loop.run(move |event, _, control_flow| {
		use winit::event::*;

		control_flow.set_poll();

		// TODO(pat.m): kinda want to pass through key/mouse up events unconditionally so tracker doesn't get stuck.
		if let Event::WindowEvent { event, .. } = &event
			&& context.egui_integration.on_event(event)
		{
			return
		}

		match event {
			Event::NewEvents(_) => {
				context.prepare_frame();
			}

			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				context.wants_quit = true;
			}

			Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
				let new_size = Vec2i::new(physical_size.width as i32, physical_size.height as i32);
				context.notify_resized(new_size);
				// app.resize(new_size);
			}

			Event::WindowEvent{ event, .. } => {
				context.input.on_window_event(&event);
			}

			Event::DeviceEvent{ event, .. } => {
				context.input.on_device_event(&event);
			}

			Event::MainEventsCleared => {
				context.start_frame();

				debug::show_menu(&mut context, &mut app, &mut debug_menu_state);
				app.present(&mut context);

				context.finalize_frame();

				if context.wants_quit {
					control_flow.set_exit();
				}
			}

			Event::LoopDestroyed => {
				context.shutdown();
			}

			_ => {}
		}
	})
}

