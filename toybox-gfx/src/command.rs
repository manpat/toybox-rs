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


