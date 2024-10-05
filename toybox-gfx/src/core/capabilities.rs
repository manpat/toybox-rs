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

	pub max_samples: usize,

	pub max_ubo_size: usize,

	pub parallel_shader_compilation_supported: bool,
}

impl Capabilities {
	pub fn from(gl: &gl::Gl) -> Self {
		let mut ubo_bind_alignment = 0;
		let mut ssbo_bind_alignment = 0;
		let mut max_user_clip_planes = 0;
		let mut max_texture_size = 0;
		let mut max_ubo_size = 0;

		let min_max_samples;
		let max_image_units;
		
		unsafe {
			// https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#subsection.6.7.1
			gl.GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut ubo_bind_alignment);
			gl.GetIntegerv(gl::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT, &mut ssbo_bind_alignment);

			gl.GetIntegerv(gl::MAX_CLIP_DISTANCES, &mut max_user_clip_planes);

			let mut max_color_texture_samples = 0;
			let mut max_depth_texture_samples = 0;
			let mut max_framebuffer_samples = 0;
			let mut max_samples = 0;
			gl.GetIntegerv(gl::MAX_COLOR_TEXTURE_SAMPLES, &mut max_color_texture_samples);
			gl.GetIntegerv(gl::MAX_DEPTH_TEXTURE_SAMPLES, &mut max_depth_texture_samples);
			gl.GetIntegerv(gl::MAX_FRAMEBUFFER_SAMPLES, &mut max_framebuffer_samples);
			gl.GetIntegerv(gl::MAX_SAMPLES, &mut max_samples);

			// This is the guaranteed minimum for any multisample compatible texture format.
			// It is overly conservative, but thats kinda fine.
			min_max_samples = max_color_texture_samples
				.min(max_depth_texture_samples)
				.min(max_framebuffer_samples)
				.min(max_samples);

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
			max_samples: min_max_samples as usize,
			max_ubo_size: max_ubo_size as usize,
			parallel_shader_compilation_supported: gl.MaxShaderCompilerThreadsARB.is_loaded(),
		}
	}
}