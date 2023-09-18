use toybox_gfx as gfx;
use crate::prelude::*;
use gfx::prelude::*;

use egui::TextureId;
use epaint::image::ImageDelta;

use gfx::core::*;
use gfx::resource_manager::*;


pub struct TextureManager {
	sampler: SamplerName,
	image: ImageName,
}

impl TextureManager {
	pub fn new(gfx: &mut gfx::System) -> TextureManager {
		let sampler = gfx.core.create_sampler();
		gfx.core.set_sampler_minify_filter(sampler, FilterMode::Nearest, None);
		gfx.core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
		gfx.core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
		gfx.core.set_debug_label(sampler, "egui sampler");

		let image = gfx.core.create_image_2d();
		gfx.core.allocate_and_upload_rgba8_image(image, Vec2i::splat(1), &[255; 4]);

		TextureManager {
			sampler,
			image,
		}
	}

	pub fn sampler(&self) -> SamplerName {
		self.sampler
	}

	pub fn image(&self) -> ImageName {
		self.image
	}

	pub fn apply_textures(&mut self, _gfx: &mut gfx::System, deltas: &[(TextureId, ImageDelta)]) {
		if deltas.is_empty() {
			return
		}

		println!("Apply {} texture deltas", deltas.len());
	}

	pub fn free_textures(&mut self, _gfx: &mut gfx::System, to_free: &[TextureId]) {
		if to_free.is_empty() {
			return
		}

		println!("Free {} texture deltas", to_free.len());
		
	}
}