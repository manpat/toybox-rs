use crate::prelude::*;
use crate::core::{Core, BufferName};

pub const UPLOAD_BUFFER_SIZE: usize = 1<<15;

pub struct UploadHeap {
	buffer_name: BufferName,

	buffer_ptr: *mut u8,
	buffer_cursor: usize,
	data_pushed_counter: usize,
	buffer_usage_counter: usize,

	frame_start_cursor: usize,
	locked_ranges: Vec<LockedRange>,

	// TODO(pat.m): separate staging from buffer management
	// staging should live in frame encoder, buffer management in resource manager
	staging_allocator: bumpalo::Bump,
	staged_uploads: Vec<StagedUpload>,
	resolved_uploads: Vec<BufferAllocation>,
}

impl UploadHeap {
	pub fn new(core: &mut Core) -> Self {
		let buffer_name = core.create_buffer();
		core.set_debug_label(buffer_name, "Upload Heap");

		let buffer_ptr;

		unsafe {
			let create_flags = gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT | gl::MAP_WRITE_BIT;
			core.gl.NamedBufferStorage(buffer_name.as_raw(), UPLOAD_BUFFER_SIZE as isize, std::ptr::null(), create_flags);

			let map_flags = gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT | gl::MAP_WRITE_BIT;
			buffer_ptr = core.gl.MapNamedBufferRange(buffer_name.as_raw(), 0, UPLOAD_BUFFER_SIZE as isize, map_flags) as *mut u8;
		};

		UploadHeap {
			buffer_name,
			buffer_ptr,
			buffer_cursor: 0,
			data_pushed_counter: 0,
			buffer_usage_counter: 0,

			frame_start_cursor: 0,
			locked_ranges: Vec::new(),

			staging_allocator: bumpalo::Bump::with_capacity(UPLOAD_BUFFER_SIZE),
			staged_uploads: Vec::new(),
			resolved_uploads: Vec::new(),
		}
	}

	pub fn reset(&mut self) {
		if self.buffer_usage_counter > UPLOAD_BUFFER_SIZE {
			dbg!(self.buffer_usage_counter);
			dbg!(self.data_pushed_counter);
			panic!("upload buffer overrun");
		}

		self.data_pushed_counter = 0;
		self.buffer_usage_counter = 0;

		self.staging_allocator.reset();
		self.staged_uploads.clear();
		self.resolved_uploads.clear();
	}

	pub fn buffer_name(&self) -> BufferName {
		self.buffer_name
	}

	fn reserve_space(&mut self, core: &mut Core, size: usize, alignment: usize) -> BufferAllocation {
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

		let allocation = BufferAllocation {
			offset,
			size,
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

	fn write_to_device<T>(&mut self, core: &mut Core, data: &[T], alignment: usize) -> BufferAllocation
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

	pub fn stage_data<T>(&mut self, data: &[T]) -> StagedUploadId
		where T: Copy + 'static
	{
		let index = self.staged_uploads.len();

		let data_copied = self.staging_allocator.alloc_slice_copy(data);

		// SAFETY: We are making a non-'static allocation 'static here.
		// This is technically a no-no, but is safe so long as references into staging_allocator
		// are banished before it is reset or dropped, and we don't call anything on staging_allocator 
		// that can view these allocations
		let bytes_static: &'static [u8] = unsafe {
			let ptr = data_copied.as_ptr();
			let byte_size = data_copied.len() * std::mem::size_of::<T>();

			std::slice::from_raw_parts(ptr.cast(), byte_size)
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

	pub fn push_to_device(&mut self, core: &mut Core) {
		core.push_debug_group("Push Upload Heap");

		// Sort descending by alignment for better packing
		let mut staged_uploads = std::mem::replace(&mut self.staged_uploads, Vec::new());
		staged_uploads.sort_by_key(|upload| !upload.alignment);

		self.resolved_uploads.resize(staged_uploads.len(), Default::default());

		for upload in staged_uploads.drain(..) {
			let allocation = self.write_to_device(core, upload.data, upload.alignment);
			self.resolved_uploads[upload.index] = allocation;
		}

		self.staged_uploads = staged_uploads;

		core.pop_debug_group();
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
	fn contains_allocation(&self, allocation: &BufferAllocation) -> bool {
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


#[derive(Copy, Clone, Debug, Default)]
pub struct BufferAllocation {
	pub offset: usize,
	pub size: usize,
}