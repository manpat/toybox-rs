use crate::bindings::BindingDescription;

pub mod draw;
pub mod dispatch;

pub use draw::{DrawArgs, DrawCmd, PrimitiveType};
pub use dispatch::DispatchArgs;


pub enum Command {
	Draw(DrawCmd),

	Compute {
		args: DispatchArgs,
		bindings: BindingDescription,
	},

	ClearBuffer,
	ClearTexture,

	CopyBuffer,
	CopyTexture,

	DebugMessage { label: String, },

	Callback(Box<dyn FnOnce(&mut crate::Core, &mut crate::ResourceManager) + 'static>),
}


impl Command {
	pub fn bindings_mut(&mut self) -> Option<&mut BindingDescription> {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, .. }) => Some(bindings),
			Compute{bindings, ..} => Some(bindings),
			_ => None
		}
	}
	
	pub fn bindings(&self) -> Option<&BindingDescription> {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, .. }) => Some(bindings),
			Compute{bindings, ..} => Some(bindings),
			_ => None
		}
	}
}