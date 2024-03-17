use crate::prelude::*;


#[derive(Debug, Clone)]
pub struct Capabilities {
	pub ubo_bind_alignment: usize,
	pub ssbo_bind_alignment: usize,

	/// Guaranteed to be at least 8
	pub max_user_clip_planes: usize,

	/// Guaranteed to be at least 16
	pub max_image_units: usize,

	/// Guaranteed to be at least 1024
	pub max_texture_size: usize,

	pub max_ubo_size: usize,
}

impl Capabilities {
	pub fn from(gl: &gl::Gl) -> Self {
		let mut ubo_bind_alignment = 0;
		let mut ssbo_bind_alignment = 0;
		let mut max_user_clip_planes = 0;
		let mut max_texture_size = 0;
		let mut max_ubo_size = 0;
		let max_image_units;
		
		unsafe {
			// https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#subsection.6.7.1
			gl.GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut ubo_bind_alignment);
			gl.GetIntegerv(gl::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT, &mut ssbo_bind_alignment);

			gl.GetIntegerv(gl::MAX_CLIP_DISTANCES, &mut max_user_clip_planes);

			let mut max_vertex_image_units = 0;
			let mut max_fragment_image_units = 0;
			let mut max_compute_image_units = 0;
			let mut max_combined_image_units = 0;
			gl.GetIntegerv(gl::MAX_VERTEX_TEXTURE_IMAGE_UNITS, &mut max_vertex_image_units);
			gl.GetIntegerv(gl::MAX_TEXTURE_IMAGE_UNITS, &mut max_fragment_image_units);
			gl.GetIntegerv(gl::MAX_COMPUTE_TEXTURE_IMAGE_UNITS, &mut max_compute_image_units);
			gl.GetIntegerv(gl::MAX_COMBINED_TEXTURE_IMAGE_UNITS, &mut max_combined_image_units);

			// This is kinda overkill since I'm never going to intentionally use more than the minimum of 16,
			// but still neat to see whats available.
			max_image_units = max_vertex_image_units.min(max_fragment_image_units).min(max_compute_image_units)
				.min(max_combined_image_units/2);

			gl.GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut max_texture_size);
			gl.GetIntegerv(gl::MAX_UNIFORM_BLOCK_SIZE, &mut max_ubo_size);
		}

		Capabilities {
			ubo_bind_alignment: ubo_bind_alignment as usize,
			ssbo_bind_alignment: ssbo_bind_alignment as usize,
			max_user_clip_planes: max_user_clip_planes as usize,
			max_image_units: max_image_units as usize,
			max_texture_size: max_texture_size as usize,
			max_ubo_size: max_ubo_size as usize,
		}
	}
}