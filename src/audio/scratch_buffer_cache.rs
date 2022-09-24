use crate::audio::scratch_buffer::ScratchBuffer;


pub(in crate::audio) struct ScratchBufferCache {
	mono_buffers: Vec<ScratchBuffer>,
	mono_allocated_index: usize,

	stereo_buffers: Vec<ScratchBuffer>,
	stereo_allocated_index: usize,
	buffer_size: usize,
}

impl ScratchBufferCache {
	pub fn new(buffer_size: usize) -> ScratchBufferCache {
		ScratchBufferCache {
			mono_buffers: Vec::new(),
			mono_allocated_index: 0,
			stereo_buffers: Vec::new(),
			stereo_allocated_index: 0,
			buffer_size,
		}
	}

	pub fn buffer_size(&self) -> usize {
		self.buffer_size
	}

	pub fn reset(&mut self, mono_buffer_count: usize, stereo_buffer_count: usize) {
		tracing::info!("ScratchBufferCache::reset! {} {}", mono_buffer_count, stereo_buffer_count);

		if self.mono_buffers.len() < mono_buffer_count {
			self.mono_buffers.resize_with(mono_buffer_count, || ScratchBuffer::new(self.buffer_size, false));
		}

		if self.stereo_buffers.len() < stereo_buffer_count {
			self.stereo_buffers.resize_with(stereo_buffer_count, || ScratchBuffer::new(self.buffer_size, true));
		}

		self.mono_allocated_index = 0;
		self.stereo_allocated_index = 0;
	}

	pub fn new_buffer(&mut self, stereo: bool) -> *mut ScratchBuffer {
		let (buffers, allocated_index) = match stereo {
			false => (&mut self.mono_buffers, &mut self.mono_allocated_index),
			true => (&mut self.stereo_buffers, &mut self.stereo_allocated_index),
		};

		assert!(*allocated_index < buffers.len());
		let buffer = &mut buffers[*allocated_index];
		*allocated_index += 1;
		buffer
	}
}