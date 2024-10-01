use crate::prelude::*;
use std::path::{Path, PathBuf};

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

	/// Automatically resize to match a fraction of the backbuffers size.
	MatchBackbufferFraction(u32),
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
	pub fn from_vfs(core: &mut Core, vfs: &vfs::Vfs, virtual_path: &Path, label: String) -> anyhow::Result<ImageResource> {
		// TODO(pat.m): use a BufReader instead so that image can read only what it needs
		let data = vfs.load_resource_data(virtual_path)?;

		let image = ::image::load_from_memory(&data)?.flipv().into_rgba8();
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

	pub fn array_from_vfs(core: &mut Core, vfs: &vfs::Vfs, virtual_paths: &[PathBuf], label: String) -> anyhow::Result<ImageResource> {
		if virtual_paths.is_empty() {
			anyhow::bail!("Trying to create empty image array")
		}

		let mut image_data = Vec::new();

		let mut common_size = None;

		for virtual_path in virtual_paths {
			// TODO(pat.m): use a BufReader instead so that image can read only what it needs
			let file_data = vfs.load_resource_data(virtual_path)?;

			let image = ::image::load_from_memory(&file_data)?.flipv().into_rgba8();
			let size = image.dimensions();

			if *common_size.get_or_insert(size) != size {
				let (w, h) = common_size.unwrap();
				let (w2, h2) = size;
				let path = virtual_path.display();
				anyhow::bail!("Size mismatch while loading image array '{label}'. Expected {w}x{h}, but {path} was {w2}x{h2}");
			}

			image_data.extend(image.into_vec());
		}

		let (width, height) = common_size.unwrap();
		let num_layers = virtual_paths.len() as u32;
		let size = Vec2i::new(width as i32, height as i32);

		// TODO(pat.m): allow diff texel formats
		let name = core.create_image_2d_array(ImageFormat::Srgba8, size, num_layers);
		core.upload_image(name, None, ImageFormat::Srgba8, &image_data);
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

		match req.resize_policy {
			ImageResizePolicy::MatchBackbuffer => {
				image_info.size = core.backbuffer_size().extend(1);
			}

			ImageResizePolicy::MatchBackbufferFraction(fraction) => {
				image_info.size = (core.backbuffer_size() / fraction as i32).extend(1);
			}

			_ => {}
		}

		let name = core.create_image_from_info(image_info.clone());
		core.set_debug_label(name, &req.label);

		match req.clear_policy {
			ImageClearPolicy::Never => {}
			ImageClearPolicy::DefaultAtFrameStart => {
				core.clear_image_to_default(name);
			}
		}

		ImageResource {
			name,
			image_info,
			resize_policy: req.resize_policy,
			clear_policy: req.clear_policy,
			label: req.label.clone(),
		}
	}

	pub(crate) fn on_resize(&mut self, core: &mut Core) {
		let size_2d = match self.resize_policy {
			ImageResizePolicy::Fixed => return,
			ImageResizePolicy::MatchBackbuffer => core.backbuffer_size(),
			ImageResizePolicy::MatchBackbufferFraction(fraction) => core.backbuffer_size() / fraction as i32,
		};

		self.image_info.size = size_2d.extend(1);

		core.destroy_image(self.name);
		self.name = core.create_image_from_info(self.image_info.clone());
		core.set_debug_label(self.name, &self.label);
	}
}




