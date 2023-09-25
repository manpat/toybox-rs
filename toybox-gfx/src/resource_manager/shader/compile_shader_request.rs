use crate::core::ShaderType;
use crate::resource_manager::*;

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub struct CompileShaderRequest {
	pub label: String,
	pub src: String,
	pub shader_type: ShaderType,
}


impl CompileShaderRequest {
	pub fn vertex(label: impl Into<String>, src: impl Into<String>) -> CompileShaderRequest {
		CompileShaderRequest {
			label: label.into(),
			src: src.into(),
			shader_type: ShaderType::Vertex,
		}
	}

	pub fn fragment(label: impl Into<String>, src: impl Into<String>) -> CompileShaderRequest {
		CompileShaderRequest {
			label: label.into(),
			src: src.into(),
			shader_type: ShaderType::Fragment,
		}
	}

	pub fn compute(label: impl Into<String>, src: impl Into<String>) -> CompileShaderRequest {
		CompileShaderRequest {
			label: label.into(),
			src: src.into(),
			shader_type: ShaderType::Compute,
		}
	}
}


impl ResourceRequest for CompileShaderRequest {
	type Resource = ShaderResource;

	fn register(self, rm: &mut ResourceManager) -> ShaderHandle {
		rm.compile_shader_requests.request_handle(&mut rm.shaders, self)
	}
}