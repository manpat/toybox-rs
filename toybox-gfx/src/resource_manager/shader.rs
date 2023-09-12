use crate::prelude::*;
use std::path::{Path, PathBuf};

use crate::core::{
	self,
	shader::{ShaderName, ShaderType},
};


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32);

impl super::ResourceHandle for ShaderHandle {
	fn from_raw(value: u32) -> Self { ShaderHandle(value) }
}


#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct ShaderDef {
	pub path: PathBuf,
	pub shader_type: ShaderType,
}

impl ShaderDef {
	pub fn from(path: impl Into<PathBuf>) -> anyhow::Result<ShaderDef> {
		let path = path.into();

		let Some(extension) = path.extension() else {
			anyhow::bail!("Path missing extension: '{}'", path.display())
		};

		if extension != "glsl" {
			anyhow::bail!("Extension must end in 'glsl': '{}'", path.display())
		}

		let Some(stem) = path.file_stem().and_then(std::ffi::OsStr::to_str) else {
			anyhow::bail!("Path missing file stem: '{}'", path.display())
		};

		let shader_type = if stem.ends_with(".vs") { ShaderType::Vertex }
			else if stem.ends_with(".fs") { ShaderType::Fragment }
			else if stem.ends_with(".cs") { ShaderType::Compute }
			else { anyhow::bail!("Unknown shader extension: '{}'", path.display()) };

		Ok(ShaderDef {
			path,
			shader_type,
		})
	}

	pub fn vertex(path: impl Into<PathBuf>) -> ShaderDef {
		ShaderDef {
			path: path.into(),
			shader_type: ShaderType::Vertex,
		}
	}

	pub fn fragment(path: impl Into<PathBuf>) -> ShaderDef {
		ShaderDef {
			path: path.into(),
			shader_type: ShaderType::Fragment,
		}
	}

	pub fn compute(path: impl Into<PathBuf>) -> ShaderDef {
		ShaderDef {
			path: path.into(),
			shader_type: ShaderType::Compute,
		}
	}
}


#[derive(Debug)]
pub struct ShaderResource {
	pub name: ShaderName,
	pub num_user_clip_planes: u32,
}

impl super::Resource for ShaderResource {
	type Handle = ShaderHandle;
	type Name = ShaderName;
	type Def = ShaderDef;

	fn get_name(&self) -> ShaderName { self.name }
}

impl ShaderResource {
	pub fn from_disk(core: &mut core::Core, shader_type: ShaderType, full_path: &Path) -> anyhow::Result<ShaderResource> {
		let data = std::fs::read_to_string(full_path)?;

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

		let label = format!("shader:{}", full_path.display());
		core.set_debug_label(name, &label);
		core.debug_marker(&label);

		Ok(ShaderResource {
			name,
			num_user_clip_planes: if uses_user_clipping { 4 } else { 0 },
		})
	}
}