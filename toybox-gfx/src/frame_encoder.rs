use crate::prelude::*;
use crate::command_group::*;
use crate::core::Core;
use crate::upload_heap::{UploadStage, StagedUploadId};
use crate::bindings::*;
use crate::arguments::*;



// Encodes per-frame commands, organised into passes/command groups
pub struct FrameEncoder {
	pub(crate) command_groups: Vec<CommandGroup>,
	pub(crate) backbuffer_clear_color: Color,

	pub upload_stage: UploadStage,

	pub global_bindings: BindingDescription,
}

impl FrameEncoder {
	pub fn new(_core: &mut Core) -> FrameEncoder {
		FrameEncoder {
			command_groups: Vec::new(),
			backbuffer_clear_color: Color::light_magenta(),

			upload_stage: UploadStage::new(),
			global_bindings: BindingDescription::new(),
		}
	}

	pub fn start_frame(&mut self) {
		// Any command or command group not specifying a framebuffer should use the default
		self.global_bindings.bind_framebuffer(FramebufferArgument::Default);
	}

	pub fn end_frame(&mut self) {
		for group in self.command_groups.iter_mut() {
			group.reset();
		}

		self.global_bindings.clear();
		self.upload_stage.reset();
	}
}


impl FrameEncoder {
	pub fn backbuffer_color(&mut self, color: impl Into<Color>) {
		self.backbuffer_clear_color = color.into();
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

	pub fn command_group<'g>(&'g mut self, stage: FrameStage) -> CommandGroupEncoder<'g> {
		let group_index = match self.command_groups.iter()
			.position(|group| group.stage == stage)
		{
			Some(index) => index,
			None => {
				self.command_groups.push(CommandGroup::new(stage));
				self.command_groups.len() - 1
			}
		};

		CommandGroupEncoder::new(&mut self.command_groups[group_index], &mut self.upload_stage)
	}
}

/// Global per-frame bindings.
impl FrameEncoder {
	pub fn bind_global_buffer(&mut self, target: impl Into<BufferBindTarget>, buffer: impl IntoBufferArgument) {
		self.global_bindings.bind_buffer(target, buffer.into_buffer_argument(&mut self.upload_stage));
	}

	pub fn bind_global_ubo(&mut self, index: u32, buffer: impl IntoBufferArgument) {
		self.bind_global_buffer(BufferBindTarget::UboIndex(index), buffer);
	}

	pub fn bind_global_ssbo(&mut self, index: u32, buffer: impl IntoBufferArgument) {
		self.bind_global_buffer(BufferBindTarget::SsboIndex(index), buffer);
	}

	pub fn bind_global_sampled_image(&mut self, unit: u32, image: impl Into<ImageArgument>, sampler: impl Into<SamplerArgument>) {
		self.global_bindings.bind_sampled_image(ImageBindTarget::Sampled(unit), image, sampler);
	}

	pub fn bind_global_image(&mut self, unit: u32, image: impl Into<ImageArgument>) {
		self.global_bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image);
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn bind_global_image_rw(&mut self, unit: u32, image: impl Into<ImageArgument>) {
		self.global_bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image);
	}
}