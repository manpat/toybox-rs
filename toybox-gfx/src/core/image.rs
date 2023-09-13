use crate::prelude::*;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageName {
	pub raw: u32,
	pub image_type: ImageType,
}

impl super::ResourceName for ImageName {
	const GL_IDENTIFIER: u32 = gl::TEXTURE;
	fn as_raw(&self) -> u32 { self.raw }
}


#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ImageType {
	Image1D = gl::TEXTURE_1D,
	Image2D = gl::TEXTURE_2D,
	Image3D = gl::TEXTURE_3D,
	Image2DArray = gl::TEXTURE_2D_ARRAY,
}


/// Images
impl super::Core {
	pub fn create_image(&self, image_type: ImageType) -> ImageName {
		ImageName {
			raw: unsafe {
				let mut name = 0;
				self.gl.CreateTextures(image_type as u32, 1, &mut name);
				name
			},

			image_type,
		}
	}

	pub fn bind_image(&self, unit: u32, name: ImageName) {
		assert!(unit < self.capabilities.max_image_units as u32);

		// TODO(pat.m): state tracking
		unsafe {
			self.gl.BindTextureUnit(unit, name.raw);
		}
	}

	pub fn destroy_image(&self, name: ImageName) {
		unsafe {
			self.gl.DeleteTextures(1, &name.raw)
		}
	}

	// TODO(pat.m): this is just enough to get moving but is ultimately a v dumb api.
	// allocation and creation can probably be tied together, since glCreateTextures requires immutable storage,
	// but separate from data upload, since we may want to go through the upload heap
	pub fn allocate_and_upload_srgba8_image(&self, name: ImageName, size: Vec2i, data: &[u8]) {
		assert!(name.image_type == ImageType::Image2D);
		assert!(data.len() == (size.x * size.y * 4) as usize);

		unsafe {
			let levels = 1; // no mips
			self.gl.TextureStorage2D(name.raw, levels, gl::SRGB8_ALPHA8, size.x, size.y);

			let (level, x, y) = (0, 0, 0);
			self.gl.TextureSubImage2D(name.raw, level, x, y, size.x, size.y, gl::RGBA, gl::UNSIGNED_BYTE, data.as_ptr() as *const _);
		}
	}
}