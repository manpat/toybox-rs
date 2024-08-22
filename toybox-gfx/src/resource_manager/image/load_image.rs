use crate::resource_manager::*;
use std::path::PathBuf;

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct LoadImageRequest {
	pub path: PathBuf,
}


impl LoadImageRequest {
	pub fn from(path: impl Into<PathBuf>) -> LoadImageRequest {
		LoadImageRequest { path: path.into() }
	}
}


impl ResourceRequest for LoadImageRequest {
	type Resource = ImageResource;

	fn register(self, rm: &mut ResourceManager) -> ImageHandle {
		rm.load_image_requests.request_handle(&mut rm.images, self)
	}
}