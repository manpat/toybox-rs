use crate::prelude::*;
use crate::core::{BufferName,ImageName};

use std::collections::HashMap;

// https://registry.khronos.org/OpenGL-Refpages/gl4/html/glMemoryBarrier.xhtml

const ALL_BUFFER_BARRIER_BITS: u32 = gl::VERTEX_ATTRIB_ARRAY_BARRIER_BIT | gl::ELEMENT_ARRAY_BARRIER_BIT
	| gl::TEXTURE_FETCH_BARRIER_BIT // Included because this can affect texture fetches from buffer textures
	| gl::COMMAND_BARRIER_BIT | gl::PIXEL_BUFFER_BARRIER_BIT | gl::UNIFORM_BARRIER_BIT
	| gl::BUFFER_UPDATE_BARRIER_BIT | gl::TRANSFORM_FEEDBACK_BARRIER_BIT | gl::ATOMIC_COUNTER_BARRIER_BIT
	| gl::SHADER_STORAGE_BARRIER_BIT;

const ALL_IMAGE_BARRIER_BITS: u32 = gl::TEXTURE_FETCH_BARRIER_BIT
	| gl::SHADER_IMAGE_ACCESS_BARRIER_BIT | gl::PIXEL_BUFFER_BARRIER_BIT
	| gl::TEXTURE_UPDATE_BARRIER_BIT | gl::FRAMEBUFFER_BARRIER_BIT;


#[derive(Debug, Default)]
pub struct BarrierTracker {
	buffers: HashMap<BufferName, u32>,
	images: HashMap<ImageName, u32>,

	next_barrier_flags: u32,
}

impl BarrierTracker {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn read_buffer(&mut self, name: BufferName, read_usage_bits: u32) {
		// If a buffer has been written to and a barrier matching read_usage_bits has not yet been emitted
		// then we need to emit one before it gets read.
		if let Some(needs_barrier_flags) = self.buffers.get(&name)
			&& *needs_barrier_flags & read_usage_bits != 0
		{
			self.next_barrier_flags |= read_usage_bits;
		}
	}

	pub fn write_buffer(&mut self, name: BufferName, read_usage_bits: u32) {
		// We assume a write hazard is _also_ a read hazard (that is all writes are actually read/writes).
		// so subsequent draw calls that write to the same buffer still need a barrier to ensure ordering.
		self.read_buffer(name, read_usage_bits);

		// Any reads from this buffer after the next draw call will require a barrier of the right type.
		// Since barriers apply for _all_ buffers, keep track of which barriers we are yet to emit for any
		// given buffer.
		self.buffers.insert(name, ALL_BUFFER_BARRIER_BITS);
	}

	pub fn read_image(&mut self, name: ImageName, read_usage_bits: u32) {
		// If a image has been written to and a barrier matching read_usage_bits has not yet been emitted
		// then we need to emit one before it gets read.
		if let Some(needs_barrier_flags) = self.images.get(&name)
			&& *needs_barrier_flags & read_usage_bits != 0
		{
			self.next_barrier_flags |= read_usage_bits;
		}
	}

	pub fn write_image(&mut self, name: ImageName, read_usage_bits: u32) {
		// We assume a write hazard is _also_ a read hazard (that is all writes are actually read/writes).
		// so subsequent draw calls that write to the same image still need a barrier to ensure ordering.
		self.read_image(name, read_usage_bits);

		// Any reads from this image after the next draw call will require a barrier of the right type.
		// Since barriers apply for _all_ images, keep track of which barriers we are yet to emit for any
		// given image.
		self.images.insert(name, ALL_IMAGE_BARRIER_BITS);
	}

	/// Inserts any barriers required for the next draw call to be well defined, assuming appropriate calls to
	/// read/write_* have been made for each resource bound.
	/// Must be called immediately before any draw call/command that may read from or write to a resource.
	pub fn emit_barriers(&mut self, gl: &gl::Gl) {
		let barrier_flags = std::mem::replace(&mut self.next_barrier_flags, 0);

		if barrier_flags != 0 {
			unsafe {
				gl.MemoryBarrier(barrier_flags);
			}
		}

		for needs_barrier_flags in self.buffers.values_mut() {
			*needs_barrier_flags &= !barrier_flags;
		}

		for needs_barrier_flags in self.images.values_mut() {
			*needs_barrier_flags &= !barrier_flags;
		}
	}

	pub fn giga_barrier(&mut self, gl: &gl::Gl) {
		unsafe {
			gl.MemoryBarrier(gl::ALL_BARRIER_BITS);	
		}

		self.next_barrier_flags = 0;
	}
}

impl super::Core {
	pub fn giga_barrier(&self) {
		self.barrier_tracker().giga_barrier(&self.gl);
	}
}