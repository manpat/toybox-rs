use crate::prelude::*;
use crate::core::*;
use crate::resource_manager::*;

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct CreateImageRequest {
	pub image_info: ImageInfo,
	pub resize_policy: ImageResizePolicy,
	pub label: String,
}


impl CreateImageRequest {
	pub fn rendertarget(label: impl Into<String>, format: ImageFormat) -> CreateImageRequest {
		CreateImageRequest {
			image_info: ImageInfo {
				image_type: ImageType::Image2D,
				format,
				size: Vec3i::zero(),
				levels: 1,
			},

			resize_policy: ImageResizePolicy::MatchBackbuffer,
			label: label.into(),
		}
	}

	pub fn fixed_2d(label: impl Into<String>, size: Vec2i, format: ImageFormat) -> CreateImageRequest {
		CreateImageRequest {
			image_info: ImageInfo {
				image_type: ImageType::Image2D,
				format,
				size: size.extend(1),
				levels: 1,
			},
			
			resize_policy: ImageResizePolicy::Fixed,
			label: label.into(),
		}
	}
}


impl ResourceRequest for CreateImageRequest {
	type Resource = ImageResource;

	fn register(self, rm: &mut ResourceManager) -> ImageHandle {
		rm.create_image_requests.request_handle(&mut rm.images, self)
	}
}