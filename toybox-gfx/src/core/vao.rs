use crate::prelude::*;

/// VAO
impl super::Core {
	pub(super) fn create_and_bind_global_vao(gl: &gl::Gl) -> u32 {
		unsafe {
			let mut name = 0;
			gl.CreateVertexArrays(1, &mut name);

			let label = "Global Vao";
			gl.ObjectLabel(gl::VERTEX_ARRAY, name, label.len() as i32, label.as_ptr() as *const _);
			gl.BindVertexArray(name);
			name
		}
	}

	pub(super) fn destroy_global_vao(&self) {
		unsafe {
			self.gl.DeleteVertexArrays(1, &self.global_vao_name);
		}
	}
}