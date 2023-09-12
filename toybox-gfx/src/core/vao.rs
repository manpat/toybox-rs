use crate::prelude::*;
use crate::core::BufferName;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VaoName(pub u32);


impl super::ResourceName for VaoName {
	const GL_IDENTIFIER: u32 = gl::VERTEX_ARRAY;
	fn as_raw(&self) -> u32 { self.0 }
}


/// VAO
impl super::Core {
	pub fn create_vao(&self) -> VaoName {
		unsafe {
			let mut name = 0;
			self.gl.CreateVertexArrays(1, &mut name);
			VaoName(name)
		}
	}

	pub fn destroy_vao(&self, name: VaoName) {
		unsafe {
			self.gl.DeleteVertexArrays(1, &name.as_raw());
		}
	}

	pub fn bind_vao(&self, name: VaoName) {
		if self.bound_vao.get() == name {
			return;
		}

		self.bound_vao.set(name);

		unsafe {
			self.gl.BindVertexArray(name.as_raw());
		}
	}

	pub fn set_vao_index_buffer(&self, name: VaoName, buffer: BufferName) {
		unsafe {
			self.gl.VertexArrayElementBuffer(name.as_raw(), buffer.as_raw());
		}
	}
}