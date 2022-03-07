#![feature(backtrace, array_chunks, array_windows, type_ascription)]

pub mod prelude;
pub mod gfx;
pub mod perf;
pub mod window;
pub mod input;
pub mod audio;
pub mod utility;
pub mod imgui_backend;

pub use crate::prelude::*;

use std::error::Error;

pub struct Engine {
	pub sdl_ctx: sdl2::Sdl,
	pub event_pump: sdl2::EventPump,
	pub window: sdl2::video::Window,
	pub gfx: gfx::Context,
	pub input: input::InputSystem,
	pub audio: audio::AudioSystem,
	pub instrumenter: perf::Instrumenter,

	pub imgui: imgui_backend::ImguiBackend,

	should_quit: bool,
}


impl Engine {
	pub fn new(window_name: &str) -> Result<Engine, Box<dyn Error>> {
		#[cfg(feature="tracy")]
		init_tracy();

		let sdl_ctx = sdl2::init()?;
		let sdl_video = sdl_ctx.video()?;
		let sdl_audio = sdl_ctx.audio()?;

		let (window, mut gfx) = window::init_window(&sdl_video, window_name)?;
		let event_pump = sdl_ctx.event_pump()?;
		let input = input::InputSystem::new(sdl_ctx.mouse(), &window);
		let audio = audio::AudioSystem::new(sdl_audio)?;

		let mut imgui = imgui_backend::ImguiBackend::new(&mut gfx)?;
		let instrumenter = perf::Instrumenter::new(&mut gfx);

		// Make sure aspect is set up correctly
		let (w, h) = window.drawable_size();
		let drawable_size = Vec2i::new(w as _, h as _);

		let (w, h) = window.size();
		let window_size = Vec2i::new(w as _, h as _);

		gfx.on_resize(drawable_size);
		imgui.on_resize(drawable_size, window_size);

		Ok(Engine {
			sdl_ctx,
			event_pump,
			window,
			gfx,
			input,
			audio,
			instrumenter,
			
			imgui,

			should_quit: false,
		})
	}

	pub fn should_quit(&self) -> bool { self.should_quit }

	#[instrument(skip_all, name="Engine::process_events")]
	pub fn process_events(&mut self) {
		self.input.clear();

		for event in self.event_pump.poll_iter() {
			use sdl2::event::{Event, WindowEvent};

			match event {
				Event::Quit {..} => { self.should_quit = true }
				Event::Window{ win_event: WindowEvent::Resized(..), .. } => {
					let (w, h) = self.window.drawable_size();
					let drawable_size = Vec2i::new(w as _, h as _);

					let (w, h) = self.window.size();
					let window_size = Vec2i::new(w as _, h as _);

					self.gfx.on_resize(drawable_size);
					self.imgui.on_resize(drawable_size, window_size);
					self.input.handle_event(&event)
				}

				_ => {
					let imgui_claimed = self.imgui.handle_event(&event);
					if !imgui_claimed {
						self.input.handle_event(&event);
					}
				},
			}
		}

		self.input.process_events();

		self.imgui.start_frame();
	}

	#[instrument(skip_all, name="Engine::end_frame")]
	pub fn end_frame(&mut self) {
		{
			let _guard = self.instrumenter.scoped_section("audio");
			self.audio.update();
		}

		self.instrumenter.end_frame();

		self.imgui.draw(&mut self.gfx);

		{
			let _guard = self.instrumenter.scoped_section("swap");
			self.window.gl_swap_window();
		}

		tracing::info!(tracy.frame_mark=true);
	}
}



#[cfg(feature="tracy")]
fn init_tracy() {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(tracing_tracy::TracyLayer::new());

    tracing::subscriber::set_global_default(subscriber)
    	.expect("set up the subscriber");
    	
	println!("tracy init");
}