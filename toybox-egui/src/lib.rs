use toybox_gfx as gfx;

use egui_winit::winit::{self, event::WindowEvent, window::Window};
use egui_winit::egui::{self, output::FullOutput};
use tracing::instrument;
use std::rc::Rc;

mod renderer;
mod textures;
mod conversions;

pub mod prelude {
	pub use egui_winit::egui;
	pub use egui::epaint;
	pub use egui::emath;

	pub use crate::conversions::*;
}

pub use textures::{image_name_to_egui, image_handle_to_egui};


pub struct Integration {
	state: egui_winit::State,
	ctx: egui::Context,
	window: Rc<Window>,

	renderer: renderer::Renderer,
	texture_manager: textures::TextureManager,
}

impl Integration {
	#[instrument(skip_all, name="egui Integration::new")]
	pub fn new(ctx: egui::Context, window: Rc<Window>, gfx: &mut gfx::System) -> anyhow::Result<Integration> {
		let theme = None;
		let scale_factor = window.scale_factor() as f32;

		let state = egui_winit::State::new(
			ctx.clone(),
			egui::ViewportId::ROOT,
			&*window,
			Some(scale_factor),
			theme,
			Some(gfx.core.capabilities().max_texture_size)
		);

		// ctx.tessellation_options_mut(|opts| {
		//     // opts.feathering = false;
		//     dbg!(opts);
		// });

		let mut renderer = renderer::Renderer::new(gfx);
		renderer.scaling = scale_factor;

		let texture_manager = textures::TextureManager::new(gfx);

		Ok(Integration {
			ctx, state, window,
			renderer, texture_manager,
		})
	}

	// Returns whether or not egui wants to consume the event
	#[instrument(skip_all, name="egui on_event")]
	pub fn on_event(&mut self, event: &WindowEvent) -> bool {
		use winit::keyboard::{Key, NamedKey};
		use winit::event::KeyEvent;

		// Only pass Tab to egui if it wants pointer or keyboard input because otherwise it consumes the key unconditionally.
		if let WindowEvent::KeyboardInput{ event: KeyEvent { logical_key: Key::Named(NamedKey::Tab), .. }, .. } = event
			&& !self.ctx.wants_keyboard_input()
			&& !self.ctx.wants_pointer_input()
		{
			return false
		}

		if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event {
			self.renderer.scaling = *scale_factor as f32;
		}

		self.state.on_window_event(&self.window, event).consumed
	}

	#[instrument(skip_all, name="egui start_frame")]
	pub fn start_frame(&mut self) -> egui::Context {
		let input = self.state.take_egui_input(&self.window);
		self.ctx.begin_frame(input);
		self.ctx.clone()
	}

	#[instrument(skip_all, name="egui end_frame")]
	pub fn end_frame(&mut self, gfx: &mut gfx::System) {
		let FullOutput{platform_output, textures_delta, shapes, pixels_per_point, ..} = self.ctx.end_frame();
		self.state.handle_platform_output(&self.window, platform_output);

		let primitives = self.ctx.tessellate(shapes, pixels_per_point);

		self.texture_manager.apply_textures(gfx, &textures_delta.set);
		self.renderer.paint_triangles(gfx, &primitives, &self.texture_manager);
		self.texture_manager.free_textures(gfx, &textures_delta.free);
	}
}



pub fn show_image_name(ui: &mut egui::Ui, name: gfx::ImageName) {
	let id = image_name_to_egui(name);

	let widget = egui::Image::new(egui::load::SizedTexture::new(id, [128.0; 2]))
		.uv([egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)]);

	ui.add(widget);
}

pub fn show_image_handle(ui: &mut egui::Ui, handle: gfx::ImageHandle) {
	let id = image_handle_to_egui(handle);

	let widget = egui::Image::new(egui::load::SizedTexture::new(id, [128.0; 2]))
		.uv([egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)]);

	ui.add(widget);
}