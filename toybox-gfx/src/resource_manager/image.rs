use crate::prelude::*;
use std::path::Path;

use crate::core::*;


mod load_image;
pub use load_image::*;



#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageHandle(pub u32);

impl super::ResourceHandle for ImageHandle {
	fn from_raw(value: u32) -> Self { ImageHandle(value) }
}


#[derive(Debug)]
pub enum ImageResizePolicy {
	/// Image is never resized for duration of its lifetime.
	Fixed,

	/// Automatically resize to match the backbuffer size.
	MatchBackbuffer,
}


#[derive(Debug)]
pub struct ImageResource {
	pub name: ImageName,

	// TODO(pat.m): use
	pub resize_policy: ImageResizePolicy,
}

impl super::Resource for ImageResource {
	type Handle = ImageHandle;
	type Name = ImageName;

	fn get_name(&self) -> ImageName { self.name }
}

impl ImageResource {
	pub fn from_disk(core: &mut Core, full_path: &Path) -> anyhow::Result<ImageResource> {
		let image = ::image::open(full_path)?.flipv().into_rgba8();
		let (width, height) = image.dimensions();
		let size = Vec2i::new(width as i32, height as i32);
		let data = image.into_vec();
		
		// TODO(pat.m): allow diff texel formats
		let name = core.create_image_2d(ImageFormat::Srgba8, size);
		core.upload_image(name, None, ImageFormat::Srgba8, &data);

		Ok(ImageResource {
			name,
			resize_policy: ImageResizePolicy::Fixed,
		})
	}
}




