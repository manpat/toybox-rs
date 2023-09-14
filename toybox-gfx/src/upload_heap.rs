use crate::prelude::*;
use crate::core::{Core, BufferName, buffer::BufferRange};

pub const UPLOAD_BUFFER_SIZE: usize = 8<<20;

pub struct UploadHeap {
	buffer_name: BufferName,

	buffer_ptr: *mut u8,
	buffer_cursor: usize,
	data_pushed_counter: usize,
	buffer_usage_counter: usize,

	frame_start_cursor: usize,
	locked_ranges: Vec<LockedRange>,

	resolved_uploads: Vec<BufferRange>,
}

impl UploadHeap {
	pub fn new(core: &mut Core) -> Self {
		let create_flags = gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT | gl::MAP_WRITE_BIT;

		let buffer_name = core.create_buffer();
		core.set_debug_label(buffer_name, "Upload Heap");
		core.allocate_buffer_storage(buffer_name, UPLOAD_BUFFER_SIZE, create_flags);

		let buffer_ptr = unsafe {
			let map_flags = gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT | gl::MAP_WRITE_BIT;
			core.gl.MapNamedBufferRange(buffer_name.as_raw(), 0, UPLOAD_BUFFER_SIZE as isize, map_flags) as *mut u8
		};

		assert!(!buffer_ptr.is_null(), "Failed to map upload heap");

		UploadHeap {
			buffer_name,
			buffer_ptr,
			buffer_cursor: 0,
			data_pushed_counter: 0,
			buffer_usage_counter: 0,

			frame_start_cursor: 0,
			locked_ranges: Vec::new(),

			resolved_uploads: Vec::new(),
		}
	}

	pub fn reset(&mut self) {
		if self.buffer_usage_counter > UPLOAD_BUFFER_SIZE {
			dbg!(self.buffer_usage_counter, UPLOAD_BUFFER_SIZE);
			dbg!(self.data_pushed_counter);
			panic!("upload buffer overrun");
		}

		self.data_pushed_counter = 0;
		self.buffer_usage_counter = 0;
		self.resolved_uploads.clear();
	}

	pub fn buffer_name(&self) -> BufferName {
		self.buffer_name
	}

	fn reserve_space(&mut self, core: &mut Core, size: usize, alignment: usize) -> BufferRange {
		// Move to next alignment boundary
		let pre_alignment_cursor = self.buffer_cursor;
		self.buffer_cursor = (self.buffer_cursor + alignment - 1) & (!alignment + 1);

		let should_invalidate = self.buffer_cursor + size > UPLOAD_BUFFER_SIZE;
		if should_invalidate {
			self.buffer_cursor = 0;
		}

		let offset = self.buffer_cursor;
		self.buffer_cursor += size;

		// Keep track of total buffer usage - including alignment
		self.buffer_usage_counter += self.buffer_cursor.checked_sub(pre_alignment_cursor)
			.unwrap_or(size + UPLOAD_BUFFER_SIZE - pre_alignment_cursor);

		let allocation = BufferRange {
			offset,

			// HACK: this is a measure to avoid binding ranges smaller than the minimum required size - specifically UBOs.
			// this is needs a bit more thinking about tho, as alignment is not necessarily the correct value to use here
			size: size.max(alignment),
		};

		// Check if we need to wait for the earliest range to be used.
		while let Some(locked_range) = self.locked_ranges.first()
			&& locked_range.contains_allocation(&allocation)
		{
			let range = self.locked_ranges.remove(0);

			unsafe {
				// Eager check to see if the fence has already been signaled
				let result = core.gl.ClientWaitSync(range.fence, gl::SYNC_FLUSH_COMMANDS_BIT, 0);
				if result != gl::ALREADY_SIGNALED && result != gl::CONDITION_SATISFIED {
					print!("Upload heap sync");

					// wait in blocks of 0.1ms
					let timeout_ns = 100_000;

					while let result = core.gl.ClientWaitSync(range.fence, gl::SYNC_FLUSH_COMMANDS_BIT, timeout_ns)
						&& result != gl::ALREADY_SIGNALED && result != gl::CONDITION_SATISFIED
					{
						print!(".");
					}

					println!("!");
				}

				core.gl.DeleteSync(range.fence);
			}
		}

		allocation
	}

	fn write_to_device<T>(&mut self, core: &mut Core, data: &[T], alignment: usize) -> BufferRange
		where T: Copy + 'static
	{
		let byte_size = data.len() * std::mem::size_of::<T>();
		let allocation = self.reserve_space(core, byte_size, alignment);

		unsafe {
			let dest_ptr = self.buffer_ptr.offset(allocation.offset as isize);
			std::ptr::copy(data.as_ptr(), dest_ptr.cast(), data.len());
		}

		self.data_pushed_counter += byte_size;

		allocation
	}

	pub fn resolve_allocation(&self, staged_upload: StagedUploadId) -> BufferRange {
		self.resolved_uploads.get(staged_upload.0).cloned()
			.expect("Invalid staged upload id")
	}

	pub fn create_end_frame_fence(&mut self, core: &mut Core) {
		let fence = unsafe {
			core.gl.FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0)
		};

		let range_size = self.buffer_cursor.checked_sub(self.frame_start_cursor)
			.unwrap_or(UPLOAD_BUFFER_SIZE - self.frame_start_cursor + self.buffer_cursor);

		self.locked_ranges.push(LockedRange {
			fence,
			start: self.frame_start_cursor,
			size: range_size,
		});

		self.frame_start_cursor = self.buffer_cursor;
	}
}






#[derive(Debug)]
struct LockedRange {
	fence: gl::types::GLsync,

	start: usize,
	size: usize, // NOTE: may wrap
}

impl LockedRange {
	fn contains_allocation(&self, allocation: &BufferRange) -> bool {
		let allocation_end = allocation.offset + allocation.size;
		let range_end = self.start + self.size;

		if range_end <= UPLOAD_BUFFER_SIZE {
			allocation.offset < range_end && allocation_end >= self.start
		} else {
			allocation.offset >= self.start || allocation_end < (range_end - UPLOAD_BUFFER_SIZE)
		}
	}
}



#[derive(Copy, Clone, Debug)]
struct StagedUpload {
	data: &'static [u8],
	alignment: usize,
	index: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StagedUploadId(usize);


pub struct UploadStage {
	staging_allocator: bumpalo::Bump,
	staged_uploads: Vec<StagedUpload>,
}

impl UploadStage {
	pub fn new() -> Self {
		UploadStage {
			staging_allocator: bumpalo::Bump::with_capacity(UPLOAD_BUFFER_SIZE),
			staged_uploads: Vec::new(),
		}
	}

	pub fn reset(&mut self) {
		self.staging_allocator.reset();
		self.staged_uploads.clear();
	}

	pub fn stage_data<U>(&mut self, data: &U) -> StagedUploadId
		where U: crate::AsStageableSlice + ?Sized
	{
		let index = self.staged_uploads.len();

		let data_copied = self.staging_allocator.alloc_slice_copy(data.as_slice());

		// SAFETY: We are making a non-'static allocation 'static here.
		// This is technically a no-no, but is safe so long as references into staging_allocator
		// are banished before it is reset or dropped, and we don't call anything on staging_allocator 
		// that can view these allocations
		let bytes_static = unsafe {
			as_static_bytes(data_copied)
		};

		self.staged_uploads.push(StagedUpload {
			data: bytes_static,
			alignment: 1,
			index,
		});

		StagedUploadId(index)
	}

	pub fn stage_data_iter<I, T>(&mut self, iter: I) -> StagedUploadId
	    where I: IntoIterator<Item = T>
		    , I::IntoIter: ExactSizeIterator
		    , T: Copy + Sized + 'static
    {
		let index = self.staged_uploads.len();

		let data_copied = self.staging_allocator.alloc_slice_fill_iter(iter);

		// SAFETY: We are making a non-'static allocation 'static here.
		// This is technically a no-no, but is safe so long as references into staging_allocator
		// are banished before it is reset or dropped, and we don't call anything on staging_allocator 
		// that can view these allocations
		let bytes_static = unsafe {
			as_static_bytes(data_copied)
		};

		self.staged_uploads.push(StagedUpload {
			data: bytes_static,
			alignment: 1,
			index,
		});

		StagedUploadId(index)
    }

	pub fn update_staged_upload_alignment(&mut self, upload_id: StagedUploadId, new_aligment: usize) {
		let Some(upload) = self.staged_uploads.get_mut(upload_id.0) else {
			panic!("Trying to update alignment with invalid staged upload id");
		};

		upload.alignment = upload.alignment.max(new_aligment);
	}

	pub fn push_to_heap(&mut self, core: &mut Core, upload_heap: &mut UploadHeap) {
		core.push_debug_group("Push Upload Heap");

		// Sort descending by alignment for better packing
		self.staged_uploads.sort_by_key(|upload| !upload.alignment);

		upload_heap.resolved_uploads.resize(self.staged_uploads.len(), Default::default());

		for upload in self.staged_uploads.drain(..) {
			let allocation = upload_heap.write_to_device(core, upload.data, upload.alignment);
			upload_heap.resolved_uploads[upload.index] = allocation;
		}

		core.pop_debug_group();
	}
}


unsafe fn as_static_bytes<T>(slice: &[T]) -> &'static [u8]
	where T: Copy + Sized + 'static
{
	unsafe {
		let ptr = slice.as_ptr();
		let byte_size = slice.len() * std::mem::size_of::<T>();
		std::slice::from_raw_parts(ptr.cast(), byte_size)
	}
}