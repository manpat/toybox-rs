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


/// A generic type that provides access to an OpenGL buffer resource.
///
/// New buffers can be created via [`gfx::ResourceContext::new_buffer`].
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


impl<T: Copy> Buffer<T> {
	pub(crate) fn from_raw(handle: u32, usage: BufferUsage) -> Buffer<T> {
		Buffer {
			handle, usage,
			length: 0,
			_phantom: PhantomData,
		}
	}

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

	/// Create a view of a single element of this buffer for binding.
	///
	/// ## Note
	/// Some buffer types have fairly strict alignment requirements, and so it is up to the user to properly align `element`.
	/// Notably UBOs can require 256B alignment for offsets.
	/// See: Capabilities::ubo_offset_alignment
	pub fn element_view(&self, element: u32) -> RangedBufferView<T> {
		RangedBufferView::new(*self, element, 1)
	}

	/// Create a view of a range of elements in this buffer for binding.
	///
	/// ## Note
	/// Some buffer types have fairly strict alignment requirements, and so it is up to the user to properly align `element`.
	/// Notably UBOs can require 256B alignment for offsets.
	/// See: Capabilities::ubo_offset_alignment
	pub fn range_view(&self, offset: u32, count: u32) -> RangedBufferView<T> {
		RangedBufferView::new(*self, offset, count)
	}
}



/// Represents a range of elements of some Buffer.
/// Used when only a subset of a buffer is to be bound to a bind point.
#[derive(Copy, Clone, Debug)]
pub struct RangedBufferView<T: Copy> {
	pub(super) handle: u32,
	offset: u32,
	count: u32,
	_phantom: PhantomData<*const T>,
}

impl<T: Copy> RangedBufferView<T> {
	pub fn new(buffer: Buffer<T>, offset: u32, count: u32) -> Self {
		assert!(offset + count <= buffer.length);
		RangedBufferView {
			handle: buffer.handle,
			offset,
			count,
			_phantom: PhantomData
		}
	}

	pub fn len(&self) -> u32 {
		self.count
	}

	pub fn offset(&self) -> u32 {
		self.offset
	}
}


impl<T: Copy> From<Buffer<T>> for RangedBufferView<T> {
	fn from(buffer: Buffer<T>) -> Self {
		RangedBufferView::new(buffer, 0, buffer.len())
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