use crate::{
	Core, Resources,
	FramebufferName,
	FramebufferDescription,
};

#[derive(Debug, Default, Clone)]
pub enum FramebufferArgument {
	#[default]
	Default,
	Name(FramebufferName),
	Description(FramebufferDescription),
}


impl<T> From<T> for FramebufferArgument
	where T: Into<FramebufferDescription>
{
	fn from(o: T) -> Self {
		FramebufferArgument::Description(o.into())
	}
}


impl From<FramebufferName> for FramebufferArgument {
	fn from(o: FramebufferName) -> Self {
		FramebufferArgument::Name(o)
	}
}

impl FramebufferArgument {
	pub fn resolve_name(&self, core: &Core, resources: &mut Resources) -> Option<FramebufferName> {
		match self {
			FramebufferArgument::Default => None,
			FramebufferArgument::Name(name) => Some(*name),
			FramebufferArgument::Description(desc) => resources.resolve_framebuffer(core, desc.clone()),
		}
	}
}