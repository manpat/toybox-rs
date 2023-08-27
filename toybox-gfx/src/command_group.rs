use crate::bindings::BindingDescription;
use crate::command::{Command, compute, draw};
use crate::resource_manager::shader::ShaderHandle;
use crate::upload_heap::{UploadStage, StagedUploadId};


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
	pub upload_stage: &'g mut UploadStage,
}

impl<'g> CommandGroupEncoder<'g> {
	pub fn new(group: &'g mut CommandGroup, upload_stage: &'g mut UploadStage) -> Self {
		CommandGroupEncoder { group, upload_stage }
	}

	pub fn add(&mut self, command: impl Into<Command>) {
		self.group.commands.push(command.into());
	}

	pub fn upload<T>(&mut self, data: &impl crate::AsSlice<Target=T>) -> StagedUploadId
		where T: Copy + 'static
	{
		self.upload_stage.stage_data(data.as_slice())
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
		draw::DrawCmdBuilder {cmd, upload_stage: self.upload_stage}
	}
}