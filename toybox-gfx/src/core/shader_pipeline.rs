use crate::prelude::*;
use super::shader::{ShaderType, ShaderName};
use super::ResourceName;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderPipelineName(pub u32);

impl super::ResourceName for ShaderPipelineName {
	const GL_IDENTIFIER: u32 = gl::PROGRAM_PIPELINE;
	fn as_raw(&self) -> u32 { self.0 }
}



/// Shader Pipelines
impl super::Core {
	pub fn create_shader_pipeline(&self) -> ShaderPipelineName {
		unsafe {
			let mut name = 0;
			self.gl.CreateProgramPipelines(1, &mut name);
			ShaderPipelineName(name)
		}
	}

	pub fn destroy_shader_pipeline(&self, name: ShaderPipelineName) {
		unsafe {
			self.gl.DeleteProgramPipelines(1, &name.0);
		}
	}

	pub fn clear_shader_pipeline(&self, name: ShaderPipelineName) {
		unsafe {
			self.gl.UseProgramStages(name.as_raw(), gl::ALL_SHADER_BITS, 0);
		}
	}

	pub fn attach_shader_to_pipeline(&self, pipeline: ShaderPipelineName, shader: super::ShaderName) {
		let stage_bit = match shader.shader_type {
			ShaderType::Vertex => gl::VERTEX_SHADER_BIT,
			ShaderType::Fragment => gl::FRAGMENT_SHADER_BIT,
			ShaderType::Compute => gl::COMPUTE_SHADER_BIT,
		};

		unsafe {
			self.gl.UseProgramStages(pipeline.as_raw(), stage_bit, shader.as_raw());
		}
	}

	pub fn bind_shader_pipeline(&self, pipeline: ShaderPipelineName) {
		if self.bound_shader_pipeline.get() != pipeline {
			unsafe {
				self.gl.BindProgramPipeline(pipeline.as_raw());
			}

			self.bound_shader_pipeline.set(pipeline);
		}
	}
}