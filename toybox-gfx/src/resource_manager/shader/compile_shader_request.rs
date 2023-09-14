use crate::core::ShaderType;

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
