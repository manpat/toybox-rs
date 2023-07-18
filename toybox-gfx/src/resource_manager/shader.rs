use crate::prelude::*;
use std::path::PathBuf;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u32);

impl super::ResourceHandle for ShaderHandle {
	fn from_raw(value: u32) -> Self { ShaderHandle(value) }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ShaderType {
	Vertex = gl::VERTEX_SHADER,
	Fragment = gl::FRAGMENT_SHADER,
	Compute = gl::COMPUTE_SHADER,
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


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderName(pub u32);

impl crate::core::ResourceName for ShaderName {
	const GL_IDENTIFIER: u32 = gl::PROGRAM;
	fn as_raw(&self) -> u32 { self.0 }
}


#[derive(Debug)]
pub struct ShaderResource {
	pub name: ShaderName,
}

impl super::Resource for ShaderResource {
	type Handle = ShaderHandle;
	type Name = ShaderName;
	type Def = ShaderDef;

	fn get_name(&self) -> ShaderName { self.name }
}