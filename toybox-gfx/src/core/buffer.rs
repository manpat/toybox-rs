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

	pub fn bind_disptach_indirect_buffer(&self, name: impl Into<Option<BufferName>>) {
		self.bind_buffer(BufferTarget::DispatchIndirect, name);
	}
}