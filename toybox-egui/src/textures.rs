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
	default_image: ImageName,

	managed_images: HashMap<TextureId, Option<ManagedImage>>,
}

#[derive(Debug)]
struct ManagedImage {
	name: ImageName,
	allocated_size: Vec2i,

	// If true, texture will be in gl::R16 format
	holds_font: bool,
}


impl TextureManager {
	#[tracing::instrument(skip_all, name="egui TextureManager::new")]
	pub fn new(gfx: &mut gfx::System) -> TextureManager {
		let sampler = gfx.core.create_sampler();
		gfx.core.set_sampler_minify_filter(sampler, FilterMode::Linear, None);
		gfx.core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
		gfx.core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
		gfx.core.set_debug_label(sampler, "egui sampler");

		let format = ImageFormat::Rgba(ComponentFormat::Unorm8);
		let default_image = gfx.core.create_image_2d(format, Vec2i::splat(1));
		gfx.core.upload_image(default_image, None, format, &[255u8, 0, 255, 255]);
		gfx.core.set_debug_label(default_image, "egui default image");

		TextureManager {
			sampler,
			default_image,

			managed_images: HashMap::new(),
		}
	}

	pub fn sampler(&self) -> SamplerName {
		self.sampler
	}

	pub fn image_from_texture_id(&self, resource_manager: &gfx::ResourceManager, id: TextureId) -> ImageName {
		if let TextureId::User(id) = id {
			let value = (id & 0xffff_ffff) as u32;
			let is_image_handle = (id & IMAGE_HANDLE_BIT) != 0;

			// Map to either an ImageName directly or to an ImageHandle that is immediately resolved.
			return match is_image_handle {
				false => unsafe {
					ImageName::from_raw(value)
				}

				true => {
					let handle = gfx::ImageHandle::from_raw(value);
					resource_manager.images.get_name(handle)
						.unwrap_or(self.default_image)
				}
			}
		}

		if let Some(Some(managed_image)) = self.managed_images.get(&id) {
			managed_image.name
		} else {
			self.default_image
		}
	}

	pub fn is_font_image(&self, id: TextureId) -> bool {
		if let Some(Some(managed_image)) = self.managed_images.get(&id) {
			managed_image.holds_font
		} else {
			false
		}
	}

	pub fn apply_textures(&mut self, gfx: &mut gfx::System, deltas: &[(TextureId, ImageDelta)]) {
		if deltas.is_empty() {
			return
		}

		for (id, delta) in deltas {
			let managed_image = self.managed_images.entry(*id)
				.or_insert_with(|| None);

			// If delta is incompatible with existing image then we need to reallocate
			if let Some(image) = managed_image
				&& !is_managed_image_compatible(image, delta)
			{
				gfx.core.destroy_image(image.name);
				*managed_image = None;
			}

			// If we're yet to allocate storage or our storage has been invalidated, create it now
			if managed_image.is_none() {
				let is_full_image_update = delta.pos.is_none();
				assert!(is_full_image_update);

				let TextureId::Managed(managed_id) = id else {
					panic!("egui trying to update user image")
				};

				let label = match delta.image {
					ImageData::Color(_) => format!("egui color image #{managed_id}"),
					ImageData::Font(_) => format!("egui font atlas #{managed_id}"),
				};

				*managed_image = Some(create_managed_image(&gfx.core, delta, label));
			}

			// By this point we must have a ready managed image, so unconditionally upload the data
			let Some(managed_image) = managed_image else { unreachable!() };
			upload_managed_image_data(&gfx.core, managed_image, delta);
		}
	}

	pub fn free_textures(&mut self, gfx: &mut gfx::System, to_free: &[TextureId]) {
		for id in to_free {
			if let Some(Some(managed_image)) = self.managed_images.remove(id) {
				gfx.core.destroy_image(managed_image.name);
			}
		}
	}
}





fn create_managed_image(core: &gfx::Core, delta: &ImageDelta, label: impl AsRef<str>) -> ManagedImage {
	let size = Vec2i::new(delta.image.width() as i32, delta.image.height() as i32);
	let holds_font = matches!(&delta.image, ImageData::Font(_));

	let format = match holds_font {
		true => ImageFormat::Red(ComponentFormat::Unorm16),
		false => ImageFormat::Srgba8,
	};

	let name = core.create_image_2d(format, size);
	core.set_debug_label(name, label);

	ManagedImage {
		name,
		allocated_size: size,
		holds_font,
	}
}

fn is_managed_image_compatible(managed_image: &ManagedImage, delta: &ImageDelta) -> bool {
	let delta_size = Vec2i::new(delta.image.width() as i32, delta.image.height() as i32);
	let is_full_image_update = delta.pos.is_none();
	let is_different_size = managed_image.allocated_size != delta_size;

	let is_size_compatible = !(is_full_image_update && is_different_size);

	let is_delta_font = matches!(&delta.image, ImageData::Font(_));
	let is_same_type = managed_image.holds_font == is_delta_font;

	is_size_compatible && is_same_type
}

fn upload_managed_image_data(core: &gfx::Core, managed_image: &mut ManagedImage, delta: &ImageDelta) {
	let [size_x, size_y] = delta.image.size();
	let [offset_x, offset_y] = delta.pos.unwrap_or([0, 0]);

	let range = ImageRange {
		size: Vec3i::new(size_x as i32, size_y as i32, 1),
		offset: Vec3i::new(offset_x as i32, offset_y as i32, 0),
	};

	match &delta.image {
		ImageData::Font(font_image) => {
			core.upload_image(managed_image.name, range, ImageFormat::Red(ComponentFormat::F32), &font_image.pixels);
		}

		ImageData::Color(color_image) => {
			core.upload_image(managed_image.name, range, ImageFormat::Srgba8, &color_image.pixels);
		}
	}

}


const IMAGE_HANDLE_BIT: u64 = 1<<32;

pub fn image_name_to_egui(name: gfx::ImageName) -> egui::TextureId {
	egui::TextureId::User(name.as_raw() as u64)
}

pub fn image_handle_to_egui(handle: gfx::ImageHandle) -> egui::TextureId {
	egui::TextureId::User(handle.0 as u64 | IMAGE_HANDLE_BIT)
}