use crate::SamplerName;


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum SamplerArgument {
	Name(SamplerName),
	Common(CommonSampler),
}

impl From<SamplerName> for SamplerArgument {
	fn from(name: SamplerName) -> Self {
		Self::Name(name)
	}
}

impl From<CommonSampler> for SamplerArgument {
	fn from(handle: CommonSampler) -> Self {
		Self::Common(handle)
	}
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CommonSampler {
	Nearest,
	Linear,
	NearestRepeat,
	LinearRepeat,
}