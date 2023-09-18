use toybox_gfx as gfx;
use crate::prelude::*;
use gfx::prelude::*;

use egui::TextureId;
use epaint::image::{ImageDelta, ImageData};

use gfx::core::*;
use gfx::resource_manager::*;

use std::collections::HashMap;


pub struct TextureManager {
	sampler: SamplerName,
	image: ImageName,

	managed_images: HashMap<TextureId, ManagedImage>,
}

struct ManagedImage {
	name: ImageName,
	allocated_size: Option<Vec2i>,
}


impl TextureManager {
	pub fn new(gfx: &mut gfx::System) -> TextureManager {
		let sampler = gfx.core.create_sampler();
		gfx.core.set_sampler_minify_filter(sampler, FilterMode::Linear, None);
		gfx.core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
		gfx.core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
		gfx.core.set_debug_label(sampler, "egui sampler");

		let image = gfx.core.create_image_2d();
		gfx.core.allocate_and_upload_rgba8_image(image, Vec2i::splat(1), &[255; 4]);

		TextureManager {
			sampler,
			image,

			managed_images: HashMap::new(),
		}
	}

	pub fn sampler(&self) -> SamplerName {
		self.sampler
	}

	pub fn image_from_texture_id(&self, id: TextureId) -> ImageName {
		if let Some(managed_image) = self.managed_images.get(&id) {
			managed_image.name
		} else {
			self.image
		}
	}

	pub fn apply_textures(&mut self, gfx: &mut gfx::System, deltas: &[(TextureId, ImageDelta)]) {
		if deltas.is_empty() {
			return
		}

		for (id, delta) in deltas {
			let managed_image = self.managed_images.entry(*id)
				.or_insert_with(|| create_managed_image(&gfx.core));

			update_managed_image(&gfx.core, managed_image, delta);
		}
	}

	pub fn free_textures(&mut self, _gfx: &mut gfx::System, to_free: &[TextureId]) {
		if to_free.is_empty() {
			return
		}

		println!("Free {} texture deltas", to_free.len());
	}
}





fn create_managed_image(core: &gfx::Core) -> ManagedImage {
	ManagedImage {
		name: core.create_image_2d(),
		allocated_size: None,
	}
}

fn update_managed_image(core: &gfx::Core, managed_image: &mut ManagedImage, delta: &ImageDelta) {
	let delta_size = Vec2i::new(delta.image.width() as i32, delta.image.height() as i32);
	let is_full_image_update = delta.pos.is_none();

	// Full texture update with new size requires new image
	if is_full_image_update && managed_image.allocated_size.is_some() && managed_image.allocated_size != Some(delta_size) {
		core.destroy_image(managed_image.name);
		managed_image.name = core.create_image_2d();
		managed_image.allocated_size = None;
	}

	if managed_image.allocated_size.is_none() {
		assert!(is_full_image_update, "Updating subimage of unallocated ManagedImage");

		unsafe {
			core.gl.TextureStorage2D(managed_image.name.as_raw(), 1, gl::SRGB8_ALPHA8, delta_size.x, delta_size.y);
		}

		managed_image.allocated_size = Some(delta_size);
	}

	let [offset_x, offset_y] = delta.pos.unwrap_or([0, 0]);
	let ImageData::Font(font_image) = &delta.image else { unimplemented!() };

	// TODO(pat.m): would be better to just use the coverage data directly
	let data: Vec<_> = font_image.srgba_pixels(None).collect();

	unsafe {
		core.gl.TextureSubImage2D(managed_image.name.as_raw(),
			0, offset_x as i32, offset_y as i32,
			delta_size.x, delta_size.y,
			gl::RGBA, gl::UNSIGNED_BYTE,
			data.as_ptr() as *const _);
	}
}