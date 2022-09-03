use crate::prelude::*;
use crate::{perf, imgui_backend, window};
use crate::utility::resource_scope;

/// The core of toybox.
pub struct Engine {
	pub sdl_ctx: sdl2::Sdl,
	pub event_pump: sdl2::EventPump,
	pub window: sdl2::video::Window,
	pub gfx: gfx::System,
	pub input: input::InputSystem,
	pub audio: audio::AudioSystem,
	pub instrumenter: perf::Instrumenter,

	pub imgui: imgui_backend::ImguiBackend,

	resource_scope_allocator: resource_scope::ResourceScopeAllocator,

	should_quit: bool,
}


impl Engine {
	pub fn new(window_name: &str) -> Result<Engine, Box<dyn Error>> {
		#[cfg(feature="tracy")]
		init_tracy();

		let resource_scope_allocator = resource_scope::ResourceScopeAllocator::new();

		let sdl_ctx = sdl2::init()?;
		let sdl_video = sdl_ctx.video()?;
		let sdl_audio = sdl_ctx.audio()?;

		let (window, gl_ctx) = window::init_window(&sdl_video, window_name)?;
		let mut gfx = gfx::System::new(gl_ctx, resource_scope_allocator.global_scope_token());

		let event_pump = sdl_ctx.event_pump()?;
		let mut input = input::InputSystem::new(sdl_ctx.mouse());
		let audio = audio::AudioSystem::new(sdl_audio)?;

		let mut resource_context = gfx.resource_context(None);
		let mut imgui = imgui_backend::ImguiBackend::new(&mut resource_context)?;
		let instrumenter = perf::Instrumenter::new(&mut resource_context);

		// Make sure aspect is set up correctly
		let (w, h) = window.drawable_size();
		let drawable_size = Vec2i::new(w as _, h as _);

		let (w, h) = window.size();
		let window_size = Vec2i::new(w as _, h as _);

		gfx.on_resize(drawable_size);
		imgui.on_resize(drawable_size, window_size);
		input.on_resize(window_size);

		Ok(Engine {
			sdl_ctx,
			event_pump,
			window,
			gfx,
			input,
			audio,
			instrumenter,
			
			imgui,

			resource_scope_allocator,

			should_quit: false,
		})
	}

	/// Whether or not the OS has requested the application close, e.g., by the user clicking the close button.
	pub fn should_quit(&self) -> bool { self.should_quit }

	/// Handles pushing window and input events to the relevant parts of the engine. 
	/// Should be called at the start of the main loop, or at the start of the frame.
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
					self.input.on_resize(window_size);
				}

				_ => {
					let imgui_claimed = !self.input.is_mouse_captured()
						&& self.imgui.handle_event(&event);

					if !imgui_claimed {
						self.input.handle_event(&event);
					}
				},
			}
		}

		self.input.process_events();

		self.imgui.start_frame();
	}

	/// Handles swapping, drawing any debug ui and finalising any audio changes.
	/// Should be the final call into the engine within a frame.
	#[instrument(skip_all, name="Engine::end_frame")]
	pub fn end_frame(&mut self) {
		{
			let _guard = self.instrumenter.scoped_section("audio");
			self.audio.update();
		}

		self.instrumenter.end_frame();

		self.imgui.draw(&mut self.gfx.draw_context());

		{
			let _guard = self.instrumenter.scoped_section("swap");
			self.window.gl_swap_window();
		}

		self.cleanup_resources();

		tracing::info!(tracy.frame_mark=true);
	}


	pub fn new_resource_scope(&mut self) -> resource_scope::ResourceScopeToken {
		let token = self.resource_scope_allocator.new_resource_scope();
		self.gfx.register_resource_scope(token.clone());
		token
	}
}


impl Engine {
	fn cleanup_resources(&mut self) {
		let to_remove = self.resource_scope_allocator.reap_dead_scopes();

		for scope_id in to_remove {
			self.gfx.cleanup_resource_scope(scope_id);
		}
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