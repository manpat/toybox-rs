use crate::prelude::*;
use crate::bindings::*;
use crate::core::{BlendMode};
use crate::command::{Command, compute, draw};
use crate::resources::{ShaderHandle, arguments::*};
use crate::upload_heap::{UploadStage, StagedUploadId};

use std::ops::{Deref, DerefMut};


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum FrameStage {
	Start,

	BeforeMain(i8),
	Main,
	AfterMain(i8),

	BeforeMainTransparent(i8),
	MainTransparent,
	AfterMainTransparent(i8),

	Postprocess,
	AfterPostprocess(i8),

	Ui(i32),

	DebugUi,
	Final,
}



// 
pub struct CommandGroup {
	pub stage: FrameStage,

	pub commands: SmallVec<[Command; 16]>,

	pub bindings: BindingDescription,
	pub blend_mode: BlendMode,
}

impl CommandGroup {
	pub(crate) fn new(stage: FrameStage) -> CommandGroup {
		CommandGroup {
			stage,
			commands: SmallVec::new(),
			bindings: BindingDescription::new(),
			blend_mode: BlendMode::PREMULTIPLIED_ALPHA,
		}
	}

	pub(crate) fn reset(&mut self) {
		self.commands.clear();
		self.bindings.clear();
		self.blend_mode = BlendMode::PREMULTIPLIED_ALPHA;
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

	pub fn upload(&mut self, data: &(impl crate::AsStageableSlice + ?Sized)) -> StagedUploadId {
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

/// Annotation
impl<'g> CommandGroupEncoder<'g> {
	pub fn annotate(self, label: impl Into<String>) -> AnnotatedCommandGroupEncoder<'g> {
		AnnotatedCommandGroupEncoder::annotate(self, label.into())
	}
}

/// Bindings shared between all commands in the group.
impl<'g> CommandGroupEncoder<'g> {
	pub fn bind_shared_buffer(&mut self, target: impl Into<BufferBindTarget>, buffer: impl IntoBufferArgument) {
		self.group.bindings.bind_buffer(target, buffer.into_buffer_argument(self.upload_stage));
	}

	pub fn bind_shared_ubo(&mut self, index: u32, buffer: impl IntoBufferArgument) {
		self.bind_shared_buffer(BufferBindTarget::UboIndex(index), buffer);
	}

	pub fn bind_shared_ssbo(&mut self, index: u32, buffer: impl IntoBufferArgument) {
		self.bind_shared_buffer(BufferBindTarget::SsboIndex(index), buffer);
	}

	pub fn bind_shared_sampled_image(&mut self, unit: u32, image: impl Into<ImageArgument>, sampler: impl Into<SamplerArgument>) {
		self.group.bindings.bind_sampled_image(ImageBindTarget::Sampled(unit), image, sampler);
	}

	pub fn bind_shared_image(&mut self, unit: u32, image: impl Into<ImageArgument>) {
		self.group.bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image);
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn bind_shared_image_rw(&mut self, unit: u32, image: impl Into<ImageArgument>) {
		self.group.bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image);
	}

	pub fn bind_rendertargets(&mut self, rts: impl Into<FramebufferArgument>)  {
		self.group.bindings.bind_framebuffer(rts);
	}
}

/// Pipeline state fallbacks for all commands in the group.
impl<'g> CommandGroupEncoder<'g> {
	pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
		self.group.blend_mode = blend_mode;
	}
}

/// Commands
impl<'g> CommandGroupEncoder<'g> {
	pub fn debug_marker(&mut self, label: impl Into<String>) {
		self.add(Command::DebugMessage {
			label: label.into()
		});
	}

	pub fn execute(&mut self, cb: impl FnOnce(&mut crate::Core, &mut crate::Resources) + 'static) {
		self.add(Command::Callback(Box::new(cb)));
	}

	pub fn draw(&mut self, vertex_shader: impl Into<ShaderArgument>, fragment_shader: impl Into<ShaderArgument>) -> draw::DrawCmdBuilder<'_> {
		self.add(draw::DrawCmd::from_shaders(vertex_shader.into(), Some(fragment_shader.into())));
		let Some(Command::Draw(cmd)) = self.group.commands.last_mut() else { unreachable!() };
		draw::DrawCmdBuilder {cmd, upload_stage: self.upload_stage}
	}

	// TODO(pat.m): draw_depth_only

	/// Same as draw() except uses standard fullscreen vertex shader [gfx::Resources::fullscreen_vs_shader].
	/// If no fragment shader is provided, uses texture only [gfx::Resources::flat_fs_shader]. 
	pub fn draw_fullscreen(&mut self, fragment_shader: impl Into<Option<ShaderHandle>>) -> draw::DrawCmdBuilder<'_> {
		let fragment_shader = fragment_shader.into()
			.map_or(CommonShader::FlatTexturedFragment.into(), |handle| handle.into());

		self.add(draw::DrawCmd::from_fullscreen_shader(fragment_shader));
		let Some(Command::Draw(cmd)) = self.group.commands.last_mut() else { unreachable!() };
		draw::DrawCmdBuilder {cmd, upload_stage: self.upload_stage}
	}

	pub fn compute(&mut self, compute_shader: impl Into<ShaderArgument>) -> compute::ComputeCmdBuilder<'_> {
		self.add(compute::ComputeCmd::new(compute_shader.into()));
		let Some(Command::Compute(cmd)) = self.group.commands.last_mut() else { unreachable!() };
		compute::ComputeCmdBuilder {cmd, upload_stage: self.upload_stage}
	}

	pub fn clear_image_to_default(&mut self, image: impl Into<ImageArgument>) {
		let image = image.into();

		self.execute(move |core, rm| {
			let name = match image {
				ImageArgument::Name(name) => name,
				ImageArgument::Handle(handle) => rm.images.get_name(handle).expect("Failed to resolve image handle"),
				ImageArgument::Blank(_) => panic!("Trying to clear a basic image - these are immutable"),
			};

			core.clear_image_to_default(name);
		});
	}
}

pub struct AnnotatedCommandGroupEncoder<'g> {
	enc: CommandGroupEncoder<'g>,
}

impl<'g> AnnotatedCommandGroupEncoder<'g> {
	fn annotate(mut enc: CommandGroupEncoder<'g>, label: String) -> Self {
		enc.add(Command::PushDebugGroup{label});
		AnnotatedCommandGroupEncoder{enc}
	}
}

impl<'g> Deref for AnnotatedCommandGroupEncoder<'g> {
	type Target = CommandGroupEncoder<'g>;
	fn deref(&self) -> &Self::Target { &self.enc }
}

impl<'g> DerefMut for AnnotatedCommandGroupEncoder<'g> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.enc }
}

impl Drop for AnnotatedCommandGroupEncoder<'_> {
	fn drop(&mut self) {
		self.enc.add(Command::PopDebugGroup);
	}
}