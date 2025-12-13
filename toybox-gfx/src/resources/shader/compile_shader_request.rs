use crate::core::ShaderType;
use crate::resources::*;

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

	fn register(self, rm: &mut Resources) -> ShaderHandle {
		rm.compile_shader_requests.request_handle(&mut rm.shaders, self)
	}
}


impl Resources {
	pub fn compile_vertex_shader(&mut self, label: impl Into<String>, src: impl Into<String>) -> ShaderHandle {
		self.request(CompileShaderRequest::vertex(label, src))
	}

	pub fn compile_fragment_shader(&mut self, label: impl Into<String>, src: impl Into<String>) -> ShaderHandle {
		self.request(CompileShaderRequest::fragment(label, src))
	}

	pub fn compile_compute_shader(&mut self, label: impl Into<String>, src: impl Into<String>) -> ShaderHandle {
		self.request(CompileShaderRequest::compute(label, src))
	}
}