#![doc = include_str!("../README.md")]
#![feature(let_chains)]

pub mod prelude;
pub use crate::prelude::*;

pub mod context;
pub use context::Context;

use host::Host;


pub trait App {
	fn customise_debug_menu(&mut self, _: &mut egui::Ui) {}
	fn present(&mut self, _: &mut Context);
}


pub struct Engine {
	host: Host,
}

impl Engine {
	pub fn create(title: &str) -> anyhow::Result<Engine> {
		let host = Host::create(title)?;
		host.install_default_error_handler();

		Ok(Engine { host })
	}

	pub fn run<F, A>(self, start_app: F) -> anyhow::Result<()>
		where A: App + 'static
			, F: FnOnce(&mut Context) -> anyhow::Result<A>
	{
		let Host{ event_loop, gl_state: gl, surface, gl_context, window, .. } = self.host;

		let window = std::rc::Rc::new(window);

		let mut gfx = {
			let core = gfx::Core::new(surface, gl_context, gl);
			gfx::System::new(core)?
		};

		let audio = audio::init()?;

		let egui = egui::Context::default();
		let egui_integration = egui_backend::Integration::new(egui.clone(), window.clone(), &mut gfx)?;

		let mut context = Context {
			gfx,
			audio,
			egui,

			egui_integration,

			show_debug_menu: false,
			wants_quit: false,
		};

		let mut debug_menu_state = DebugMenuState::default();

		let mut app = start_app(&mut context)?;

		event_loop.run(move |event, _, control_flow| {
			use winit::event::*;

			control_flow.set_poll();

			if let Event::WindowEvent { event, .. } = &event
				&& context.egui_integration.on_event(event)
			{
				return
			}

			match event {
				Event::NewEvents(_) => {
					context.prepare_frame();
				}

				Event::WindowEvent { event: WindowEvent::CloseRequested, .. }
				| Event::DeviceEvent {
					event: DeviceEvent::Key(KeyboardInput{ virtual_keycode: Some(VirtualKeyCode::Escape), .. }), .. } => {
					context.wants_quit = true;
				}

				Event::DeviceEvent { event: DeviceEvent::Key(KeyboardInput{
					virtual_keycode: Some(VirtualKeyCode::F10),
					state: ElementState::Pressed,
					..
				}), .. } => {
					context.show_debug_menu = !context.show_debug_menu;
				}

				Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
					let new_size = Vec2i::new(physical_size.width as i32, physical_size.height as i32);
					context.notify_resized(new_size);
					// app.resize(new_size);
				}

				Event::MainEventsCleared => {
					context.start_frame();
					show_debug_menu(&mut context, &mut app, &mut debug_menu_state);
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
}




pub fn run<F, A>(title: &str, start_app: F) -> anyhow::Result<()>
	where A: App + 'static
		, F: FnOnce(&mut Context) -> anyhow::Result<A>
{
	Engine::create(title)?
		.run(start_app)
}


// https://www.egui.rs/#demo

#[derive(Default, Copy, Clone)]
struct DebugMenuState {
	egui_settings: bool,
	egui_style: bool,

	egui_memory: bool,
	egui_textures: bool,
	egui_inspection: bool,
}

fn show_debug_menu(ctx: &mut Context, app: &mut impl App, state: &mut DebugMenuState) {
	use egui::menu;

	egui::TopBottomPanel::top("main_debug_menu")
		.show_animated(&ctx.egui, ctx.show_debug_menu, |ui| {
			menu::bar(ui, |ui| {
				ui.menu_button("Toybox", |ui| {
					show_egui_debug_menu(ui, state);

					if ui.button("Quit").clicked() {
						ctx.wants_quit = true;
					}
				});

				app.customise_debug_menu(ui);
			})
		});

	egui::Window::new("Egui Settings")
		.open(&mut state.egui_settings)
		.show(&ctx.egui, |ui| {
			ctx.egui.settings_ui(ui);
		});

	egui::Window::new("Egui Style")
		.open(&mut state.egui_style)
		.show(&ctx.egui, |ui| {
			ctx.egui.style_ui(ui);
		});

	egui::Window::new("Egui Memory")
		.open(&mut state.egui_memory)
		.show(&ctx.egui, |ui| {
			ctx.egui.memory_ui(ui);
		});

	egui::Window::new("Egui Textures")
		.open(&mut state.egui_textures)
		.show(&ctx.egui, |ui| {
			ctx.egui.texture_ui(ui);
		});

	egui::Window::new("Egui Inspection")
		.open(&mut state.egui_inspection)
		.show(&ctx.egui, |ui| {
			ctx.egui.inspection_ui(ui);
		});
}

fn show_egui_debug_menu(ui: &mut egui::Ui, state: &mut DebugMenuState) {
	ui.menu_button("Egui", |ui| {
		ui.toggle_value(&mut state.egui_settings, "Settings");
		ui.toggle_value(&mut state.egui_style, "Style");

		ui.toggle_value(&mut state.egui_memory, "Memory");
		ui.toggle_value(&mut state.egui_textures, "Textures");
		ui.toggle_value(&mut state.egui_inspection, "Inspection");
	});
}