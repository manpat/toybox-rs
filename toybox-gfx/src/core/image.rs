use crate::prelude::*;
use super::buffer::*;

mod format;
pub use format::*;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageName {
	raw: u32,
}

impl ImageName {
	pub unsafe fn from_raw(raw: u32) -> ImageName {
		ImageName{raw}
	}
}

impl super::ResourceName for ImageName {
	const GL_IDENTIFIER: u32 = gl::TEXTURE;
	fn as_raw(&self) -> u32 { self.raw }
}


#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ImageType {
	Image2D = gl::TEXTURE_2D,
	Image3D = gl::TEXTURE_3D,
	Image2DArray = gl::TEXTURE_2D_ARRAY,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageInfo {
	pub image_type: ImageType,
	pub format: ImageFormat,
	pub size: Vec3i,
}


/// Images
impl super::Core {
	pub fn create_image_from_info(&self, image_info: ImageInfo) -> ImageName {
		let mut name = 0;
		let levels = 1;

		let size = image_info.size;
		let format = image_info.format;

		unsafe {
			self.gl.CreateTextures(image_info.image_type as u32, 1, &mut name);

			match image_info.image_type {
				ImageType::Image2D => {
					self.gl.TextureStorage2D(name, levels, format.to_raw(), size.x, size.y)
				}

				ImageType::Image3D | ImageType::Image2DArray => {
					self.gl.TextureStorage3D(name, levels, format.to_raw(), size.x, size.y, size.z)
				}
			}
		};

		let name = ImageName {raw: name};
		self.image_info.borrow_mut().insert(name, image_info);
		name
	}

	pub fn create_typed_image(&self, image_type: ImageType, format: ImageFormat, size: Vec3i) -> ImageName {
		self.create_image_from_info(ImageInfo{image_type, format, size})
	}

	pub fn create_image_2d(&self, format: ImageFormat, size: Vec2i) -> ImageName {
		self.create_typed_image(ImageType::Image2D, format, size.extend(1))
	}

	pub fn create_image_3d(&self, format: ImageFormat, size: Vec3i) -> ImageName {
		self.create_typed_image(ImageType::Image3D, format, size)
	}

	pub fn create_image_2d_array(&self, format: ImageFormat, size: Vec2i, layers: u32) -> ImageName {
		self.create_typed_image(ImageType::Image2DArray, format, size.extend(layers as i32))
	}

	pub fn get_image_info(&self, name: ImageName) -> Option<ImageInfo> {
		self.image_info.borrow().get(&name).cloned()
	}

	pub fn bind_sampled_image(&self, unit: u32, name: ImageName) {
		assert!(unit < self.capabilities.max_image_units as u32);

		// TODO(pat.m): state tracking
		unsafe {
			self.gl.BindTextureUnit(unit, name.raw);
		}
	}

	// TODO(pat.m): this api is both underpowered and sucks
	pub fn bind_image(&self, unit: u32, name: ImageName) {
		assert!(unit < self.capabilities.max_image_units as u32);

		// TODO(pat.m): make sure name is compatible with image binding - e.g., srgb formats are NOT SUPPORTED!
		let info = self.get_image_info(name).expect("Invalid ImageName");
		let format = info.format.to_raw();

		// TODO(pat.m): state tracking
		unsafe {
			let (level, layered, layer) = (0, gl::FALSE, 0);
			self.gl.BindImageTexture(unit, name.raw, level, layered, layer, gl::READ_ONLY, format);
		}
	}

	// TODO(pat.m): this api is both underpowered and sucks
	pub fn bind_image_rw(&self, unit: u32, name: ImageName) {
		assert!(unit < self.capabilities.max_image_units as u32);

		// TODO(pat.m): make sure name is compatible with image binding - e.g., srgb formats are NOT SUPPORTED!
		let info = self.get_image_info(name).expect("Invalid ImageName");
		let format = info.format.to_raw();

		// TODO(pat.m): state tracking
		unsafe {
			let (level, layered, layer) = (0, gl::FALSE, 0);
			self.gl.BindImageTexture(unit, name.raw, level, layered, layer, gl::READ_WRITE, format);
		}
	}

	pub fn destroy_image(&self, name: ImageName) {
		unsafe {
			self.gl.DeleteTextures(1, &name.raw)
		}

		self.image_info.borrow_mut().remove(&name);
	}

	pub unsafe fn upload_image_raw(&self, name: ImageName, range: impl Into<Option<ImageRange>>,
		format: ImageFormat, data_ptr: *const u8, data_size: usize)
	{
		let Some(image_info) = self.get_image_info(name)
			else { panic!("Trying to upload data for invalid ImageName") };

		let ImageRange {offset, size} = range.into().unwrap_or(ImageRange::from_size(image_info.size));

		let expected_size = format.texel_byte_size() * (size.x * size.y) as usize;
		assert_eq!(data_size, expected_size, "Core::upload_image_raw not passed expected amount of data");

		// TODO(pat.m): assert that size + offset < image_info.size

		unsafe {
			self.gl.PixelStorei(gl::UNPACK_ALIGNMENT, 1);
		}

		let level = 0;

		match image_info.image_type {
			ImageType::Image2D => unsafe {
				assert!(offset.z == 0);
				self.gl.TextureSubImage2D(name.as_raw(), level,
					offset.x, offset.y,
					size.x, size.y,
					format.to_raw_unsized(),
					format.to_raw_component(),
					data_ptr.cast());
			}

			ImageType::Image3D | ImageType::Image2DArray => unsafe {
				self.gl.TextureSubImage3D(name.as_raw(), level,
					offset.x, offset.y, offset.z,
					size.x, size.y, size.z,
					format.to_raw_unsized(),
					format.to_raw_component(),
					data_ptr.cast());
			}
		}
	}

	pub fn upload_image<T>(&self, name: ImageName, range: impl Into<Option<ImageRange>>,
		format: ImageFormat, data: &[T])
		where T: Copy
	{
		// TODO(pat.m): Make this conditional and actually track state properly
		self.bind_image_upload_buffer(None);

		// TODO(pat.m): SAFETY CHECKS!!!!
		// How do we know T is the right size or is bitwise compatible?
		// Can we check convertibility from T to component type?
		unsafe {
			let byte_size = data.len() * std::mem::size_of::<T>();
			self.upload_image_raw(name, range, format, data.as_ptr().cast(), byte_size);
		}
	}

	pub fn copy_image_from_buffer(&self, image_name: ImageName, 
		dest_range: impl Into<Option<ImageRange>>,
		buffer_format: ImageFormat, buffer_name: BufferName, buffer_range: impl Into<Option<BufferRange>>)
	{
		self.bind_image_upload_buffer(buffer_name);

		if let Some(BufferRange {offset, size}) = buffer_range.into() {
			unsafe {
				self.upload_image_raw(image_name, dest_range, buffer_format,
					offset as *const u8, size);
			}
		} else {
			// TODO(pat.m): what to do here. we could hack it and just calculate _a_ size here
			// but it would probably be better to keep track of how big buffers are and use that instead.
			let size = unimplemented!();

			unsafe {
				self.upload_image_raw(image_name, dest_range, buffer_format,
					std::ptr::null(), size);
			}
		}

		self.bind_image_upload_buffer(None);
	}

	// TODO(pat.m): clear_image with other formats
	pub fn clear_image_to_default(&self, image_name: ImageName) {
		let Some(info) = self.get_image_info(image_name) else { return };

		if info.format.is_depth() {
			self.clear_image_with_raw(image_name, gl::DEPTH_COMPONENT, gl::FLOAT, 1.0f32);

		} else if info.format.is_stencil() {
			self.clear_image_with_raw(image_name, gl::STENCIL_INDEX, gl::UNSIGNED_BYTE, 0u8);

		} else if info.format.is_depth_stencil() {
			self.clear_image_with_depth_stencil(image_name, 1.0, 0);

		} else if info.format.is_normalized() {
			self.clear_image_with_raw(image_name, gl::RGBA, gl::UNSIGNED_BYTE, [0u8, 0, 0, 0]);

		} else {
			self.clear_image_with_raw(image_name, gl::RGBA_INTEGER, gl::UNSIGNED_BYTE, [0u8, 0, 0, 0]);
		}
	}

	pub fn clear_image_with_f32(&self, image_name: ImageName, value: f32) {
		let Some(info) = self.get_image_info(image_name) else { return };

		let format = match info.format.is_depth() {
			true => gl::DEPTH_COMPONENT,
			false => gl::RED,
		};

		self.clear_image_with_raw(image_name, format, gl::FLOAT, value);
	}

	pub fn clear_image_with_color(&self, image_name: ImageName, value: Color) {
		self.clear_image_with_raw(image_name, gl::RGBA, gl::FLOAT, value);
	}

	pub fn clear_image_with_depth_stencil(&self, image_name: ImageName, depth: f32, stencil: u8) {
		#[repr(C)]
		#[derive(Copy, Clone)]
		struct D32_S8 {
			depth: f32,
			_empty: [u8; 3],
			stencil: u8
		}

		let clear_value = D32_S8 { depth, stencil, _empty: [0; 3] };

		self.clear_image_with_raw(image_name, gl::DEPTH_STENCIL, gl::FLOAT_32_UNSIGNED_INT_24_8_REV, clear_value);
	}

	pub fn clear_image_with_raw<T>(&self, image_name: ImageName, format: u32, data_type: u32, data: T)
		where T: Copy
	{
		let level = 0;

		unsafe {
			self.gl.ClearTexImage(image_name.as_raw(), level, format, data_type, (&data as *const T).cast());
		}
	}
}



#[derive(Copy, Clone, Debug)]
pub struct ImageRange {
	pub offset: Vec3i,
	pub size: Vec3i,
}

impl ImageRange {
	pub fn from_size(size: Vec3i) -> ImageRange {
		ImageRange {
			offset: Vec3i::zero(),
			size,
		}
	}

	pub fn from_2d_range(offset: Vec2i, size: Vec2i) -> ImageRange {
		ImageRange {
			offset: offset.extend(0),
			size: size.extend(1),
		}
	}
}

// TODO(pat.m): when Aabb3i exists
// impl From<Aabb3i> for ImageRange {}