use crate::bindings::BindingDescription;


pub enum Command {
	Draw {
		args: DrawArgs,
		bindings: BindingDescription,
	},

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



pub struct DrawArgs {}
pub struct DispatchArgs {}
