use crate::prelude::*;
use std::marker::PhantomData;


/// A hint to the driver about how this buffer will be used.
#[derive(Copy, Clone, Debug)]
pub enum BufferUsage {
	/// Contents will be updated infrequently, and are mostly unchanging.
	/// Prefer for buffers that will only ever be modified once and used many times.
	/// e.g., static level geometry, or lookup tables.
	Static,

	/// Contents will be updated frequently.
	/// Prefer for buffers that will be updated ever frame.
	/// e.g., for streaming geometry.
	Stream,

	/// Contents will be modified frequently.
	/// This one is here primarily for completeness - its really only appropriate
	/// for persistently mapped buffers which aren't wrapped yet.
	Dynamic,
}


#[derive(Copy, Clone, Debug)]
pub struct UntypedBuffer {
	pub(super) handle: u32,
	pub(super) size_bytes: usize,
	pub(super) usage: BufferUsage,
}


/// A generic type that manages an OpenGL buffer resource.
///
/// New buffers can be created via [`gfx::Context::new_buffer`].
/// `T` can be any [`Copy`] type, but it is up to client to ensure proper alignment and layout.
/// If `T` is a struct type, it is strongly encouraged to at least mark it `#[repr(C)]`.
///
/// ## Note
/// This is not an RAII type - no attempt is made to clean up the managed buffer.
#[derive(Copy, Clone, Debug)]
pub struct Buffer<T: Copy> {
	pub(super) handle: u32,
	length: u32,
	usage: BufferUsage,
	_phantom: PhantomData<*const T>,
}


impl UntypedBuffer {
	pub fn upload<T: Copy>(&mut self, data: &[T]) {
		upload_untyped(self.handle, data, self.usage);
		self.size_bytes = data.len() * std::mem::size_of::<T>();
	}

	pub fn upload_single<T: Copy>(&mut self, data: &T) {
		upload_untyped(self.handle, std::slice::from_ref(data), self.usage);
		self.size_bytes = std::mem::size_of::<T>();
	}

	pub fn into_typed<T: Copy>(self) -> Buffer<T> {
		Buffer {
			handle: self.handle,
			length: (self.size_bytes / std::mem::size_of::<T>()) as u32,
			usage: self.usage,
			_phantom: PhantomData,
		}
	}
}


impl<T: Copy> Buffer<T> {
	pub fn upload(&mut self, data: &[T]) {
		upload_untyped(self.handle, data, self.usage);
		self.length = data.len() as u32;
	}

	pub fn upload_single(&mut self, data: &T) {
		upload_untyped(self.handle, std::slice::from_ref(data), self.usage);
		self.length = 1;
	}

	pub fn len(&self) -> u32 {
		self.length
	}

	pub fn is_empty(&self) -> bool {
		self.length == 0
	}
}



impl<T: Copy> From<Buffer<T>> for UntypedBuffer {
	fn from(Buffer{handle, length, usage, ..}: Buffer<T>) -> UntypedBuffer {
		UntypedBuffer {
			handle,
			size_bytes: length as usize * std::mem::size_of::<T>(),
			usage,
		}
	}
}



#[instrument(skip_all, name="gfx::buffer::upload_untyped")]
fn upload_untyped<T: Copy>(handle: u32, data: &[T], usage: BufferUsage) {
	if data.is_empty() {
		// TODO(pat.m): is this what I want? 
		return
	}

	let usage = match usage {
		BufferUsage::Static => gfx::raw::STATIC_DRAW,
		BufferUsage::Dynamic => gfx::raw::DYNAMIC_DRAW,
		BufferUsage::Stream => gfx::raw::STREAM_DRAW,
	};

	let size_bytes = data.len() * std::mem::size_of::<T>();

	unsafe {
		gfx::raw::NamedBufferData(
			handle,
			size_bytes as _,
			data.as_ptr() as *const _,
			usage
		);
	}
}