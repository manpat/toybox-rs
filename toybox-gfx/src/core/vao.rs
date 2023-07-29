use crate::prelude::*;


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
		unsafe {
			self.gl.BindVertexArray(name.as_raw());
		}
	}
}