use crate::prelude::*;
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

#[derive(Debug, Clone)]
pub struct BufferInfo {
	pub size: usize,
	pub usage: u32,

	// TODO(pat.m): do we want to track whether or not a buffer is mapped so we can ensure
	// its storage is not discarded while it is mapped?
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
		self.buffer_info.borrow_mut().remove(&name);

		unsafe {
			self.gl.DeleteBuffers(1, &name.as_raw());
		}
	}

	// TODO(pat.m): make usage better
	pub fn allocate_buffer_storage(&self, name: BufferName, size: usize, usage: u32) {
		self.buffer_info.borrow_mut().insert(name, BufferInfo {size, usage});
		
		if size == 0 {
			return
		}

		unsafe {
			self.gl.NamedBufferStorage(name.as_raw(), size as isize, std::ptr::null(), usage);
		}
	}

	// TODO(pat.m): replace with copy from upload heap?
	pub fn upload_immutable_buffer_immediate<T>(&self, name: BufferName, data: &[T])
		where T: Copy + 'static
	{
		let usage = 0;
		let size = data.len() * std::mem::size_of::<T>();

		self.buffer_info.borrow_mut().insert(name, BufferInfo {size, usage});

		if size == 0 {
			return
		}

		unsafe {
			self.gl.NamedBufferStorage(name.as_raw(), size as isize, data.as_ptr().cast(), usage);
		}
	}

	pub fn get_buffer_info(&self, name: BufferName) -> Option<BufferInfo> {
		self.buffer_info.borrow().get(&name).cloned()
	}

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

	/// SAFETY: May return null pointer if buffer fails to map.
	/// Also valid usage of the returned pointer heavily depends on the usage
	/// flags specified in allocate_buffer_storage. Using the mapped pointer
	/// in a way that conflicts with those flags may be UB.
	/// It is also up to the client to properly synchronise reads and writes with the device to avoid races.
	pub unsafe fn map_buffer(&self, name: BufferName, range: impl Into<Option<BufferRange>>) -> *mut u8 {
		let buffer_info = self.get_buffer_info(name)
			.filter(|bi| bi.size > 0)
			.expect("Trying to map buffer with no storage");

		let BufferRange{offset, size} = range.into()
			.unwrap_or(BufferRange{offset: 0, size: buffer_info.size});

		assert!(size + offset <= buffer_info.size, "Trying to map buffer with out of bounds range");

		// TODO(pat.m): will we ever want to map with a different usage
		// than what was specified on creation?
		let map_flags = buffer_info.usage;
		unsafe {
			self.gl.MapNamedBufferRange(name.as_raw(), offset as isize, size as isize, map_flags).cast()
		}
	}

	/// SAFETY: Will invalidate the pointer returned from an earlier call to map_buffer.
	/// Using that pointer after the mapped buffer is unmapped is undefined behaviour.
	pub unsafe fn unmap_buffer(&self, name: BufferName) {
		unsafe {
			self.gl.UnmapNamedBuffer(name.as_raw());
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