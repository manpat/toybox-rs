use crate::bindings::BindingDescription;
use crate::command::Command;


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

	pub fn add(&mut self, command: Command) {
		self.group.commands.push(command);
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
}