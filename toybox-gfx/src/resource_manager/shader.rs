use crate::prelude::*;
use std::path::Path;

use crate::core::{
	self,
	shader::{ShaderName, ShaderType},
};

mod load_shader_request;
mod compile_shader_request;

pub use load_shader_request::LoadShaderRequest;
pub use compile_shader_request::CompileShaderRequest;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32);

impl super::ResourceHandle for ShaderHandle {
	fn from_raw(value: u32) -> Self { ShaderHandle(value) }
}


#[derive(Debug)]
pub struct ShaderResource {
	pub name: ShaderName,
	pub num_user_clip_planes: u32,
}

impl super::Resource for ShaderResource {
	type Handle = ShaderHandle;
	type Name = ShaderName;

	fn get_name(&self) -> ShaderName { self.name }
}

impl ShaderResource {
	pub fn from_source(core: &mut core::Core, shader_type: ShaderType, data: &str, label: &str) -> anyhow::Result<ShaderResource> {
		// TODO(pat.m): ugh
		let uses_user_clipping = data.contains("gl_ClipDistance");

		let std_output_block = match shader_type {
			ShaderType::Vertex => {
				if uses_user_clipping {
					// TODO(pat.m): fixed clip distances is no bueno
					"out gl_PerVertex { vec4 gl_Position; float gl_ClipDistance[4]; float gl_PointSize; };"
				} else {
					"out gl_PerVertex { vec4 gl_Position; float gl_PointSize; };"
				}
			}
			_ => "",
		};

		let ubo_options = "layout(row_major, std140) uniform;";
		let ssbo_options = "layout(row_major, std430) buffer;";

		let name = core.create_shader(shader_type, &[
			"#version 450",
			ubo_options,
			ssbo_options,
			std_output_block,
			&data
		])?;

		core.set_debug_label(name, &label);
		core.debug_marker(&label);

		Ok(ShaderResource {
			name,
			num_user_clip_planes: if uses_user_clipping { 4 } else { 0 },
		})
	}

	pub fn from_disk(core: &mut core::Core, shader_type: ShaderType, full_path: &Path) -> anyhow::Result<ShaderResource> {
		let label = format!("shader:{}", full_path.display());
		let data = std::fs::read_to_string(full_path)?;
		Self::from_source(core, shader_type, &data, &label)
	}
}