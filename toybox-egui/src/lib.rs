use toybox_gfx as gfx;

use egui_winit::winit::{event::WindowEvent, window::Window};
use egui_winit::egui::{self, output::FullOutput};
use std::rc::Rc;

mod renderer;
mod textures;

pub mod prelude {
    pub use egui_winit::egui;
    pub use egui::epaint;
    pub use egui::emath;
}


pub struct Integration {
    state: egui_winit::State,
    ctx: egui::Context,
    window: Rc<Window>,

    renderer: renderer::Renderer,
    texture_manager: textures::TextureManager,
}

impl Integration {
    pub fn new(ctx: egui::Context, window: Rc<Window>, gfx: &mut gfx::System) -> anyhow::Result<Integration> {
        let mut state = egui_winit::State::new(&*window);
        state.set_max_texture_side(gfx.core.capabilities().max_texture_size);
        state.set_pixels_per_point(window.scale_factor() as f32);

        // ctx.tessellation_options_mut(|opts| {
        //     // opts.feathering = false;
        //     dbg!(opts);
        // });

        let renderer = renderer::Renderer::new(gfx);
        let texture_manager = textures::TextureManager::new(gfx);
        Ok(Integration { ctx, state, window, renderer, texture_manager })
    }

    // Returns whether or not egui wants to consume the event
    pub fn on_event(&mut self, event: &WindowEvent<'_>) -> bool {
        self.state.on_event(&self.ctx, event).consumed
    }

    pub fn start_frame(&mut self) -> egui::Context {
        let input = self.state.take_egui_input(&self.window);
        self.ctx.begin_frame(input);
        self.ctx.clone()
    }

    pub fn end_frame(&mut self, gfx: &mut gfx::System) {
        let FullOutput{platform_output, textures_delta, shapes, ..} = self.ctx.end_frame();
        self.state.handle_platform_output(&self.window, &self.ctx, platform_output);

        let primitives = self.ctx.tessellate(shapes);

        self.texture_manager.apply_textures(gfx, &textures_delta.set);
        self.renderer.paint_triangles(gfx, &primitives, &self.texture_manager);
        self.texture_manager.free_textures(gfx, &textures_delta.free);
    }
}