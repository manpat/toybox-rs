use crate::{
	Core, ResourceManager,
	FramebufferName,
	FramebufferDescription,
};

#[derive(Debug, Clone)]
pub enum FramebufferArgument {
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
	pub fn resolve_name(&self, core: &Core, resource_manager: &mut ResourceManager) -> Option<FramebufferName> {
		match self {
			FramebufferArgument::Default => None,
			FramebufferArgument::Name(name) => Some(*name),
			FramebufferArgument::Description(desc) => resource_manager.resolve_framebuffer(core, desc.clone()),
		}
	}
}