use crate::command_group::{CommandGroup, CommandGroupEncoder};
use crate::command::Command;
use crate::core;


// Encodes per-frame commands, organised into passes/command groups
pub struct FrameEncoder {
	pub(crate) command_groups: Vec<CommandGroup>,
	pub(crate) backbuffer_clear_color: common::Color,
}

impl FrameEncoder {
	pub fn new(core: &mut core::Core) -> FrameEncoder {
		FrameEncoder {
			command_groups: Vec::new(),
			backbuffer_clear_color: [1.0, 0.5, 1.0].into(),
		}
	}

	pub fn reset(&mut self) {
		for group in self.command_groups.iter_mut() {
			group.reset();
		}
	}
}


impl FrameEncoder {
	pub fn backbuffer_color(&mut self, color: impl Into<common::Color>) {
		self.backbuffer_clear_color = color.into();
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

		CommandGroupEncoder::new(&mut self.command_groups[group_index])
	}
}