use crate::command_group::{CommandGroup, CommandGroupEncoder};
use crate::command::Command;
use crate::core;
use crate::upload_heap::{UploadStage, StagedUploadId};
use crate::bindings::BindingDescription;


// Encodes per-frame commands, organised into passes/command groups
pub struct FrameEncoder {
	pub(crate) command_groups: Vec<CommandGroup>,
	pub(crate) backbuffer_clear_color: common::Color,

	// TODO(pat.m): maybe this could be moved to resource manager
	pub upload_stage: UploadStage,

	pub global_bindings: BindingDescription,
}

impl FrameEncoder {
	pub fn new(_core: &mut core::Core) -> FrameEncoder {
		FrameEncoder {
			command_groups: Vec::new(),
			backbuffer_clear_color: [1.0, 0.5, 1.0].into(),

			upload_stage: UploadStage::new(),
			global_bindings: BindingDescription::new(),
		}
	}

	pub fn end_frame(&mut self, _core: &mut core::Core) {
		for group in self.command_groups.iter_mut() {
			group.reset();
		}

		self.global_bindings.clear();
		self.upload_stage.reset();
	}
}


impl FrameEncoder {
	pub fn backbuffer_color(&mut self, color: impl Into<common::Color>) {
		self.backbuffer_clear_color = color.into();
	}

	pub fn upload<T>(&mut self, data: &impl crate::AsSlice<Target=T>) -> StagedUploadId
		where T: Copy + 'static
	{
		self.upload_stage.stage_data(data.as_slice())
	}

	pub fn command_group<'g>(&'g mut self, id: &str) -> CommandGroupEncoder<'g> {
		let group_index = match self.command_groups.iter()
			.position(|group| group.label() == id)
		{
			Some(index) => index,
			None => {
				self.command_groups.push(CommandGroup::new(id.to_string()));
				self.command_groups.len() - 1
			}
		};

		CommandGroupEncoder::new(&mut self.command_groups[group_index], &mut self.upload_stage)
	}
}