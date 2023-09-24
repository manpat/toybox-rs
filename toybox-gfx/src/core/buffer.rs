use crate::prelude::*;
use super::shader::{ShaderType, ShaderName};
use super::ResourceName;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferName(pub u32);

impl super::ResourceName for BufferName {
	const GL_IDENTIFIER: u32 = gl::BUFFER;
	fn as_raw(&self) -> u32 { self.0 }
}

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum IndexedBufferTarget {
	ShaderStorage = gl::SHADER_STORAGE_BUFFER,
	Uniform = gl::UNIFORM_BUFFER,
}


#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum BufferTarget {
	DispatchIndirect = gl::DISPATCH_INDIRECT_BUFFER,
	DrawIndirect = gl::DRAW_INDIRECT_BUFFER,
	ImageUpload = gl::PIXEL_UNPACK_BUFFER,
	ImageDownload = gl::PIXEL_PACK_BUFFER,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct BufferRange {
	pub offset: usize,
	pub size: usize,
}


/// Buffers
impl super::Core {
	pub fn create_buffer(&self) -> BufferName {
		unsafe {
			let mut name = 0;
			self.gl.CreateBuffers(1, &mut name);
			BufferName(name)
		}
	}

	pub fn destroy_buffer(&self, name: BufferName) {
		unsafe {
			self.gl.DeleteBuffers(1, &name.as_raw());
		}
	}

	// TODO(pat.m): make usage better
	pub fn allocate_buffer_storage(&self, name: BufferName, size: usize, usage: u32) {
		unsafe {
			self.gl.NamedBufferStorage(name.as_raw(), size as isize, std::ptr::null(), usage);
		}
	}

	// TODO(pat.m): buffer mapping

	pub fn bind_indexed_buffer(&self, target: IndexedBufferTarget, index: u32,
		name: impl Into<Option<BufferName>>, range: impl Into<Option<BufferRange>>)
	{
		if let Some(BufferRange{offset, size}) = range.into() {
			unsafe {
				self.gl.BindBufferRange(target as u32, index, name.into().as_raw(),
					offset as isize, size as isize);
			}
		} else {
			unsafe {
				self.gl.BindBufferBase(target as u32, index, name.into().as_raw());
			}
		}
	}

	// TODO(pat.m): track if ImageUpload or ImageDownload bind points are bound so affected
	// calls can ensure they behave as expected. e.g., upload_sub_image won't work properly
	// with ImageUpload bound
	pub fn bind_buffer(&self, target: BufferTarget, name: impl Into<Option<BufferName>>) {
		unsafe {
			self.gl.BindBuffer(target as u32, name.into().as_raw());
		}
	}
}

/// Buffer Shorthands
impl super::Core {
	pub fn bind_ubo(&self, index: u32, name: impl Into<Option<BufferName>>,
		range: impl Into<Option<BufferRange>>)
	{
		self.bind_indexed_buffer(IndexedBufferTarget::Uniform, index, name, range);
	}

	pub fn bind_ssbo(&self, index: u32, name: impl Into<Option<BufferName>>,
		range: impl Into<Option<BufferRange>>)
	{
		self.bind_indexed_buffer(IndexedBufferTarget::ShaderStorage, index, name, range);
	}

	pub fn bind_draw_indirect_buffer(&self, name: impl Into<Option<BufferName>>) {
		self.bind_buffer(BufferTarget::DrawIndirect, name);
	}

	pub fn bind_dispatch_indirect_buffer(&self, name: impl Into<Option<BufferName>>) {
		self.bind_buffer(BufferTarget::DispatchIndirect, name);
	}

	pub fn bind_image_upload_buffer(&self, name: impl Into<Option<BufferName>>) {
		self.bind_buffer(BufferTarget::ImageUpload, name);
	}

	pub fn bind_image_download_buffer(&self, name: impl Into<Option<BufferName>>) {
		self.bind_buffer(BufferTarget::ImageDownload, name);
	}

	pub fn bind_index_buffer(&self, name: impl Into<Option<BufferName>>) {
		let name = name.into();

		if self.bound_index_buffer.get() != name {
			unsafe {
				self.gl.VertexArrayElementBuffer(self.global_vao_name, name.as_raw());
			}

			self.bound_index_buffer.set(name);
		}
	}
}