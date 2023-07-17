use crate::core;
use std::path::{Path, PathBuf};

// Create/Destroy api for gpu resources
// Load/Cache resources from disk
// Render target/FBO/temporary image cahage
//  - cache of images for use as single-frame render targets, automatically resized
//  - cache of images for use as single-frame image resources -  fixed size
//  - cache of FBOs for render passes
// Shader cache
pub struct ResourceManager {
	resource_root_path: PathBuf,

	shader_counter: u32,

	resize_request: Option<common::Vec2i>,
}

impl ResourceManager {
	pub fn new(_: &mut core::Core) -> ResourceManager {
		let resource_root_path = PathBuf::from("resource");

		ResourceManager {
			resource_root_path,
			shader_counter: 0,

			resize_request: None,
		}
	}

	pub fn request_resize(&mut self, new_size: common::Vec2i) {
		self.resize_request = Some(new_size);
	}

	/// Attempt to turn requested resources into committed GPU resources.
	pub fn process_requests(&mut self, _: &mut core::Core) -> anyhow::Result<()> {
		if let Some(_size) = self.resize_request.take() {
			// TODO(pat.m): Resize textures, recreate framebuffers etc
			println!("RESIZE {_size:?}");
		}

		Ok(())
	}
}


/// Request api
impl ResourceManager {

}