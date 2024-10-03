use crate::core::*;
use crate::resource_manager::*;

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct CreateImageRequest {
	pub image_info: ImageInfo,
	pub resize_policy: ImageResizePolicy,
	pub clear_policy: ImageClearPolicy,
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
				samples: 1,
			},

			resize_policy: ImageResizePolicy::MatchBackbuffer,
			clear_policy: ImageClearPolicy::DefaultAtFrameStart,
			label: label.into(),
		}
	}

	pub fn fractional_rendertarget(label: impl Into<String>, format: ImageFormat, fraction: u32) -> CreateImageRequest {
		CreateImageRequest::rendertarget(label, format)
			.resize_to_backbuffer_fraction(fraction)
	}

	pub fn fixed_2d(label: impl Into<String>, size: Vec2i, format: ImageFormat) -> CreateImageRequest {
		CreateImageRequest {
			image_info: ImageInfo {
				image_type: ImageType::Image2D,
				format,
				size: size.extend(1),
				levels: 1,
				samples: 1,
			},

			resize_policy: ImageResizePolicy::Fixed,
			clear_policy: ImageClearPolicy::Never,
			label: label.into(),
		}
	}
}

impl CreateImageRequest {
	pub fn clear_policy(self, clear_policy: ImageClearPolicy) -> Self {
		Self { clear_policy, .. self }
	}

	pub fn resize_policy(self, resize_policy: ImageResizePolicy) -> Self {
		Self { resize_policy, .. self }
	}

	pub fn resize_to_backbuffer(self) -> Self {
		self.resize_policy(ImageResizePolicy::MatchBackbuffer)
	}

	pub fn resize_to_backbuffer_fraction(self, fraction: u32) -> Self {
		self.resize_policy(ImageResizePolicy::MatchBackbufferFraction(fraction))
	}
}


impl ResourceRequest for CreateImageRequest {
	type Resource = ImageResource;

	fn register(self, rm: &mut ResourceManager) -> ImageHandle {
		rm.create_image_requests.request_handle(&mut rm.images, self)
	}
}