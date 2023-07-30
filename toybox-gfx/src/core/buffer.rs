use crate::prelude::*;
use super::shader::{ShaderType, ShaderName};
use super::ResourceName;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferName(pub u32);

impl super::ResourceName for BufferName {
	const GL_IDENTIFIER: u32 = gl::BUFFER;
	fn as_raw(&self) -> u32 { self.0 }
}



/// Shader Pipelines
impl super::Core {
	pub fn create_buffer(&self) -> BufferName {
		unsafe {
			let mut name = 0;
			self.gl.CreateBuffers(1, &mut name);
			BufferName(name)
		}
	}

	pub fn destroy_buffer(&self, name: BufferName) {
		unsafe {
			self.gl.DeleteBuffers(1, &name.as_raw());
		}
	}
}