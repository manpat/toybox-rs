use crate::prelude::*;
use std::path::Path;

use crate::core::*;


mod load_image;
mod create_image;
pub use load_image::*;
pub use create_image::*;



#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageHandle(pub u32);

impl super::ResourceHandle for ImageHandle {
	fn from_raw(value: u32) -> Self { ImageHandle(value) }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageResizePolicy {
	/// Image is never resized for duration of its lifetime.
	Fixed,

	/// Automatically resize to match the backbuffer size.
	MatchBackbuffer,
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageClearPolicy {
	/// Image is never cleared unless user explicitly clears it.
	Never,

	/// Image is cleared to default at beginning of the frame.
	DefaultAtFrameStart,

	// TODO(pat.m): Clear to value
	// TODO(pat.m): Clear on acquire for temporary images - e.g., throwaway depth buffers
}

#[derive(Debug)]
pub struct ImageResource {
	pub name: ImageName,
	pub image_info: ImageInfo,
	pub resize_policy: ImageResizePolicy,
	pub clear_policy: ImageClearPolicy,
	pub label: String,
}

impl super::Resource for ImageResource {
	type Handle = ImageHandle;
	type Name = ImageName;

	fn get_name(&self) -> ImageName { self.name }
}

impl ImageResource {
	pub fn from_disk(core: &mut Core, full_path: &Path, label: String) -> anyhow::Result<ImageResource> {
		let image = ::image::open(full_path)?.flipv().into_rgba8();
		let (width, height) = image.dimensions();
		let size = Vec2i::new(width as i32, height as i32);
		let data = image.into_vec();

		// TODO(pat.m): allow diff texel formats
		let name = core.create_image_2d(ImageFormat::Srgba8, size);
		core.upload_image(name, None, ImageFormat::Srgba8, &data);
		core.set_debug_label(name, &label);

		Ok(ImageResource {
			name,
			image_info: core.get_image_info(name).unwrap(),
			resize_policy: ImageResizePolicy::Fixed,
			clear_policy: ImageClearPolicy::Never,
			label,
		})
	}

	pub fn from_create_request(core: &mut Core, req: &CreateImageRequest) -> ImageResource {
		let mut image_info = req.image_info.clone();

		if req.resize_policy == ImageResizePolicy::MatchBackbuffer {
			image_info.size = core.backbuffer_size().extend(1);
		}
		
		let name = core.create_image_from_info(image_info.clone());
		core.set_debug_label(name, &req.label);

		ImageResource {
			name,
			image_info,
			resize_policy: req.resize_policy,
			clear_policy: req.clear_policy,
			label: req.label.clone(),
		}
	}

	pub(crate) fn on_resize(&mut self, core: &mut Core) {
		if self.resize_policy == ImageResizePolicy::Fixed {
			return
		}

		self.image_info.size = core.backbuffer_size().extend(1);

		core.destroy_image(self.name);
		self.name = core.create_image_from_info(self.image_info.clone());
		core.set_debug_label(self.name, &self.label);
	}
}




