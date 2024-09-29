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



#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct LoadImageArrayRequest {
	pub paths: Vec<PathBuf>,
	pub label: String,
}


impl LoadImageArrayRequest {
	pub fn from<P: Into<PathBuf>>(label: impl Into<String>, paths: impl IntoIterator<Item=P>) -> LoadImageArrayRequest {
		LoadImageArrayRequest {
			label: label.into(),
			paths: paths.into_iter().map(Into::into).collect()
		}
	}
}


impl ResourceRequest for LoadImageArrayRequest {
	type Resource = ImageResource;

	fn register(self, rm: &mut ResourceManager) -> ImageHandle {
		rm.load_image_array_requests.request_handle(&mut rm.images, self)
	}
}