use crate::bindings::BindingDescription;
use crate::command::{Command, draw, dispatch};
use crate::resource_manager::shader::ShaderHandle;


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
}

impl<'g> CommandGroupEncoder<'g> {
	pub fn new(group: &'g mut CommandGroup) -> Self {
		CommandGroupEncoder { group }
	}

	pub fn add(&mut self, command: impl Into<Command>) -> &mut Command {
		self.group.commands.push(command.into());
		self.group.commands.last_mut().unwrap()
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
		let Command::Draw(cmd) = self.add(draw::DrawCmd::from_shaders(vertex_shader, fragment_shader)) else { unreachable!() };
		draw::DrawCmdBuilder {cmd}
	}
}