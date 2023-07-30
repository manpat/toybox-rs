use crate::bindings::BindingDescription;
use crate::command::{Command, draw, dispatch};
use crate::resource_manager::shader::ShaderHandle;
use crate::upload_heap::UploadHeap;


// 
pub struct CommandGroup {
	pub label: String,

	pub commands: Vec<Command>,

	pub shared_bindings: BindingDescription,
}

impl CommandGroup {
	pub(crate) fn new(label: String) -> CommandGroup {
		CommandGroup {
			label,
			commands: Vec::new(),
			shared_bindings: BindingDescription::new(),
		}
	}

	pub(crate) fn reset(&mut self) {
		self.commands.clear();
		self.shared_bindings.clear();
	}
}

impl CommandGroup {
	pub fn label(&self) -> &str {
		&self.label
	}
}




pub struct CommandGroupEncoder<'g> {
	group: &'g mut CommandGroup,
	pub upload_heap: &'g mut UploadHeap,
}

impl<'g> CommandGroupEncoder<'g> {
	pub fn new(group: &'g mut CommandGroup, upload_heap: &'g mut UploadHeap) -> Self {
		CommandGroupEncoder { group, upload_heap }
	}

	pub fn add(&mut self, command: impl Into<Command>) {
		self.group.commands.push(command.into());
	}
}


impl<'g> CommandGroupEncoder<'g> {
	pub fn debug_marker(&mut self, label: impl Into<String>) {
		self.add(Command::DebugMessage {
			label: label.into()
		});
	}

	pub fn execute(&mut self, cb: impl FnOnce(&mut crate::Core, &mut crate::ResourceManager) + 'static) {
		self.add(Command::Callback(Box::new(cb)));
	}

	pub fn draw(&mut self, vertex_shader: ShaderHandle, fragment_shader: ShaderHandle) -> draw::DrawCmdBuilder<'_> {
		self.add(draw::DrawCmd::from_shaders(vertex_shader, fragment_shader));
		let Some(Command::Draw(cmd)) = self.group.commands.last_mut() else { unreachable!() };
		draw::DrawCmdBuilder {cmd, upload_heap: self.upload_heap}
	}
}