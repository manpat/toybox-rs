use crate::bindings::{BindingDescription, BufferBindTarget, IntoBufferBindSourceOrStageable, ImageBindTarget, ImageBindSource};
use crate::command::{Command, compute, draw};
use crate::resource_manager::shader::ShaderHandle;
use crate::upload_heap::{UploadStage, StagedUploadId};
use crate::core::SamplerName;


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

	pub fn upload(&mut self, data: &impl crate::AsStageableSlice) -> StagedUploadId {
		self.upload_stage.stage_data(data.as_slice())
	}

	pub fn upload_iter<T, I>(&mut self, iter: I) -> StagedUploadId
		where I: IntoIterator<Item=T>
			, I::IntoIter: ExactSizeIterator
			, T: Copy + 'static
	{
		self.upload_stage.stage_data_iter(iter)
	}
}

/// Bindings shared between all commands in the group.
impl<'g> CommandGroupEncoder<'g> {
	pub fn bind_shared_buffer(&mut self, target: impl Into<BufferBindTarget>, buffer: impl IntoBufferBindSourceOrStageable) {
		self.group.shared_bindings.bind_buffer(target, buffer.into_bind_source(self.upload_stage));
	}

	pub fn bind_shared_ubo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) {
		self.bind_shared_buffer(BufferBindTarget::UboIndex(index), buffer);
	}

	pub fn bind_shared_ssbo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) {
		self.bind_shared_buffer(BufferBindTarget::SsboIndex(index), buffer);
	}

	pub fn bind_shared_sampled_image(&mut self, unit: u32, image: impl Into<ImageBindSource>, sampler: SamplerName) {
		self.group.shared_bindings.bind_image(ImageBindTarget::Sampled(unit), image, sampler);
	}

	pub fn bind_shared_image(&mut self, unit: u32, image: impl Into<ImageBindSource>) {
		self.group.shared_bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image, None);
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn bind_shared_image_rw(&mut self, unit: u32, image: impl Into<ImageBindSource>) {
		self.group.shared_bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image, None);
	}
}

/// Commands
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

	pub fn compute(&mut self, compute_shader: ShaderHandle) -> compute::ComputeCmdBuilder<'_> {
		self.add(compute::ComputeCmd::new(compute_shader));
		let Some(Command::Compute(cmd)) = self.group.commands.last_mut() else { unreachable!() };
		compute::ComputeCmdBuilder {cmd, upload_stage: self.upload_stage}
	}
}