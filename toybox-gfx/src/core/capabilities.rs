use crate::prelude::*;


#[derive(Debug, Clone)]
pub struct Capabilities {
	pub ubo_bind_alignment: usize,
	pub ssbo_bind_alignment: usize,

	/// Guaranteed to be at least 8
	pub max_user_clip_planes: usize,
}

impl Capabilities {
	pub fn from(gl: &gl::Gl) -> Self {
		let mut ubo_bind_alignment = 0;
		let mut ssbo_bind_alignment = 0;
		let mut max_user_clip_planes = 0;
		unsafe {
			// https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#subsection.6.7.1
			gl.GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut ubo_bind_alignment);
			gl.GetIntegerv(gl::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT, &mut ssbo_bind_alignment);

			gl.GetIntegerv(gl::MAX_CLIP_DISTANCES, &mut max_user_clip_planes);
		}

		Capabilities {
			ubo_bind_alignment: ubo_bind_alignment as usize,
			ssbo_bind_alignment: ssbo_bind_alignment as usize,
			max_user_clip_planes: max_user_clip_planes as usize,
		}
	}
}