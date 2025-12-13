use crate::core::ShaderType;
use crate::resources::*;
use std::path::PathBuf;


#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct LoadShaderRequest {
	pub path: PathBuf,
	pub shader_type: ShaderType,
}

impl LoadShaderRequest {
	pub fn from(path: impl Into<PathBuf>) -> anyhow::Result<LoadShaderRequest> {
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

		Ok(LoadShaderRequest {
			path,
			shader_type,
		})
	}

	pub fn vertex(path: impl Into<PathBuf>) -> LoadShaderRequest {
		LoadShaderRequest {
			path: path.into(),
			shader_type: ShaderType::Vertex,
		}
	}

	pub fn fragment(path: impl Into<PathBuf>) -> LoadShaderRequest {
		LoadShaderRequest {
			path: path.into(),
			shader_type: ShaderType::Fragment,
		}
	}

	pub fn compute(path: impl Into<PathBuf>) -> LoadShaderRequest {
		LoadShaderRequest {
			path: path.into(),
			shader_type: ShaderType::Compute,
		}
	}
}


impl ResourceRequest for LoadShaderRequest {
	type Resource = ShaderResource;

	fn register(self, rm: &mut Resources) -> ShaderHandle {
		rm.load_shader_requests.request_handle(&mut rm.shaders, self)
	}
}

impl Resources {
	pub fn load_vertex_shader(&mut self, path: impl Into<PathBuf>) -> ShaderHandle {
		self.request(LoadShaderRequest::vertex(path))
	}

	pub fn load_fragment_shader(&mut self, path: impl Into<PathBuf>) -> ShaderHandle {
		self.request(LoadShaderRequest::fragment(path))
	}

	pub fn load_compute_shader(&mut self, path: impl Into<PathBuf>) -> ShaderHandle {
		self.request(LoadShaderRequest::compute(path))
	}
}