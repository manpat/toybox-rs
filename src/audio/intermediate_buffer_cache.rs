use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::node_graph::NodeKey;


struct InUseBuffer {
	buffer: IntermediateBuffer,

	associated_node: NodeKey,

	// set to the number of outgoing edges of a node, decremented for every use
	// reclaimed once it reaches zero
	uses: usize,
}


pub(in crate::audio) struct IntermediateBufferCache {
	unused_buffers: Vec<IntermediateBuffer>,

	// List of active buffers, sorted by associated_node.
	in_use_buffers: Vec<InUseBuffer>,

	buffer_size: usize,
}

impl IntermediateBufferCache {
	pub fn new(buffer_size: usize) -> IntermediateBufferCache {
		IntermediateBufferCache {
			unused_buffers: Vec::new(),
			in_use_buffers: Vec::new(),
			buffer_size,
		}
	}

	pub fn buffer_size(&self) -> usize {
		self.buffer_size
	}

	pub fn new_buffer(&mut self, stereo: bool) -> IntermediateBuffer {
		let mut buffer = self.unused_buffers.pop()
			.unwrap_or_else(|| IntermediateBuffer::new());

		buffer.reformat(self.buffer_size, stereo);
		buffer
	}

	pub fn get_buffer(&self, associated_node: NodeKey) -> Option<&IntermediateBuffer> {
		self.in_use_buffer_position(associated_node)
			.ok()
			.map(|position| &self.in_use_buffers[position].buffer)
	}

	pub fn post_buffer(&mut self, associated_node: NodeKey, buffer: IntermediateBuffer, uses: usize) {
		// If there are no uses then it will never be collected, so collect immediately
		if uses == 0 {
			self.unused_buffers.push(buffer);
			return;
		}

		let in_use_buffer = InUseBuffer {
			buffer,
			associated_node,
			uses,
		};

		match self.in_use_buffer_position(associated_node) {
			Ok(position) => {
				let prev_buffer = std::mem::replace(&mut self.in_use_buffers[position], in_use_buffer);
				self.unused_buffers.push(prev_buffer.buffer);
			}

			Err(position) => {
				self.in_use_buffers.insert(position, in_use_buffer);
			}
		}
	}

	pub fn mark_used(&mut self, associated_node: NodeKey) {
		if let Ok(position) = self.in_use_buffer_position(associated_node) {
			let in_use_buffer = &mut self.in_use_buffers[position];

			in_use_buffer.uses = in_use_buffer.uses.saturating_sub(1);
			if in_use_buffer.uses == 0 {
				let in_use_buffer = self.in_use_buffers.remove(position);
				self.unused_buffers.push(in_use_buffer.buffer);
			}
		}
	}

	pub fn mark_all_unused(&mut self) {
		for in_use_buffer in self.in_use_buffers.drain(..) {
			self.unused_buffers.push(in_use_buffer.buffer);
		}
	}

	fn in_use_buffer_position(&self, associated_node: NodeKey) -> Result<usize, usize> {
		self.in_use_buffers.binary_search_by_key(&associated_node, |InUseBuffer{associated_node, ..}| *associated_node)
	}
}



