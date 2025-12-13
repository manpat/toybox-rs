use crate::{
	ShaderHandle,
};


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum ShaderArgument {
	Handle(ShaderHandle),
	Common(CommonShader),
}

impl From<ShaderHandle> for ShaderArgument {
	fn from(handle: ShaderHandle) -> Self {
		Self::Handle(handle)
	}
}

impl From<CommonShader> for ShaderArgument {
	fn from(shader: CommonShader) -> Self {
		Self::Common(shader)
	}
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CommonShader {
	StandardVertex,
	FullscreenVertex,

	FlatUntexturedFragment,
	FlatTexturedFragment,
}

