use crate::audio::intermediate_buffer::IntermediateBuffer;
use crate::audio::node_graph::NodeKey;

use std::collections::HashMap;


struct InUseBuffer {
	buffer: IntermediateBuffer,

	// set to the number of outgoing edges of a node, decremented for every use
	// reclaimed once it reaches zero
	uses: usize,
}


pub(in crate::audio) struct IntermediateBufferCache {
	unused_buffers: Vec<IntermediateBuffer>,
	in_use_buffers: HashMap<NodeKey, InUseBuffer>,

	buffer_size: usize,
}

impl IntermediateBufferCache {
	pub fn new(buffer_size: usize) -> IntermediateBufferCache {
		IntermediateBufferCache {
			unused_buffers: Vec::new(),
			in_use_buffers: HashMap::new(),
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
		self.in_use_buffers.get(&associated_node)
			.map(|InUseBuffer{buffer, ..}| buffer)
	}

	pub fn post_buffer(&mut self, associated_node: NodeKey, buffer: IntermediateBuffer, uses: usize) {
		// If there are no uses then it will never be collected, so collect immediately
		if uses == 0 {
			self.unused_buffers.push(buffer);
			return;
		}

		let in_use_buffer = InUseBuffer {
			buffer,
			uses,
		};

		if let Some(prev_buffer) = self.in_use_buffers.insert(associated_node, in_use_buffer) {
			self.unused_buffers.push(prev_buffer.buffer);
		}
	}

	pub fn mark_used(&mut self, associated_node: NodeKey) {
		if let Some(in_use_buffer) = self.in_use_buffers.get_mut(&associated_node) {
			in_use_buffer.uses = in_use_buffer.uses.saturating_sub(1);
			if in_use_buffer.uses == 0 {
				let in_use_buffer = self.in_use_buffers.remove(&associated_node).unwrap();
				self.unused_buffers.push(in_use_buffer.buffer);
			}
		}
	}

	pub fn mark_all_unused(&mut self) {
		for (_, in_use_buffer) in self.in_use_buffers.drain() {
			self.unused_buffers.push(in_use_buffer.buffer);
		}
	}

	pub fn total_buffer_count(&self) -> usize {
		self.unused_buffers.len() + self.in_use_buffers.len()
	}
}



