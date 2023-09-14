#![doc = include_str!("../README.md")]
#![feature(let_chains)]

pub mod prelude;
pub use crate::prelude::*;

pub mod context;
pub use context::Context;

use host::Host;


pub trait App {
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
		};

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
					control_flow.set_exit();
				}

				Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
					let new_size = Vec2i::new(physical_size.width as i32, physical_size.height as i32);
					context.notify_resized(new_size);
					// app.resize(new_size);
				}

				Event::MainEventsCleared => {
					context.start_frame();
					app.present(&mut context);
					context.finalize_frame();
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