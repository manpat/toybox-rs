use crate::prelude::*;
use crate::bindings::*;

use crate::{
	Core, ResourceManager,
	upload_heap::UploadStage,
	arguments::*,
};

#[derive(Debug)]
pub enum DispatchSize {
	Explicit(Vec3i),
	Indirect(BufferArgument),
	DeriveFromImage(ImageArgument),
}


#[derive(Debug)]
pub struct ComputeCmd {
	pub compute_shader: ShaderArgument,
	pub dispatch_size: DispatchSize,
	pub bindings: BindingDescription,
}

impl From<ComputeCmd> for super::Command {
	fn from(o: ComputeCmd) -> Self {
		Self::Compute(o)
	}
}

impl ComputeCmd {
	pub fn new(compute_shader: ShaderArgument) -> ComputeCmd {
		ComputeCmd {
			compute_shader,
			dispatch_size: DispatchSize::Explicit(Vec3i::splat(1)),
			bindings: Default::default(),
		}
	}

	#[tracing::instrument(skip_all, name="ComputeCmd::execute")]
	pub fn execute(&self, core: &mut Core, rm: &mut ResourceManager) {
		let shader_handle = match self.compute_shader {
			ShaderArgument::Handle(handle) => handle,
			ShaderArgument::Common(shader) => rm.get_common_shader(shader),
		};

		let pipeline = rm.resolve_compute_pipeline(core, shader_handle);
		core.bind_shader_pipeline(pipeline);

		self.bindings.bind(core, rm);

		let mut barrier_tracker = core.barrier_tracker();

		match self.dispatch_size {
			DispatchSize::Explicit(size) => unsafe {
				barrier_tracker.emit_barriers(&core.gl);
				core.gl.DispatchCompute(size.x as u32, size.y as u32, size.z as u32);
			}

			DispatchSize::Indirect(bind_source) => {
				let BufferArgument::Name{name, range} = bind_source
					else { panic!("Unresolved buffer bind source description") };

				let offset = range.map(|r| r.offset).unwrap_or(0);

				core.bind_dispatch_indirect_buffer(name);

				barrier_tracker.read_buffer(name, gl::COMMAND_BARRIER_BIT);
				barrier_tracker.emit_barriers(&core.gl);

				unsafe {
					core.gl.DispatchComputeIndirect(offset as isize);
				}
			}

			DispatchSize::DeriveFromImage(bind_source) => {
				let image_name = match bind_source {
					ImageArgument::Name(name) => name,
					ImageArgument::Handle(handle) => rm.images.get_name(handle).expect("Failed to resolve image handle"),
					ImageArgument::Blank(image) => rm.get_blank_image(image),
				};

				let workgroup_size = rm.shaders.get_resource(shader_handle)
					.unwrap()
					.workgroup_size
					.expect("Compute shader resource missing workgroup size");

				let info = core.get_image_info(image_name).expect("Couldn't get image info for Compute group size");
				let image_size = info.size;

				// Round up to next multiple of workgroup_size
				let num_workgroups = (image_size + workgroup_size - Vec3i::splat(1)) / workgroup_size;

				barrier_tracker.emit_barriers(&core.gl);
				unsafe {
					core.gl.DispatchCompute(
						num_workgroups.x as u32,
						num_workgroups.y as u32,
						num_workgroups.z as u32
					);
				}
			}
		}
	}
}

pub struct ComputeCmdBuilder<'cg> {
	pub(crate) cmd: &'cg mut ComputeCmd,
	pub(crate) upload_stage: &'cg mut UploadStage,
}

impl<'cg> ComputeCmdBuilder<'cg> {
	pub fn groups(&mut self, num_groups: impl Into<Vec3i>) -> &mut Self {
		self.cmd.dispatch_size = DispatchSize::Explicit(num_groups.into());
		self
	}

	pub fn indirect(&mut self, buffer: impl IntoBufferArgument) -> &mut Self {
		self.cmd.dispatch_size = DispatchSize::Indirect(buffer.into_buffer_argument(self.upload_stage));
		self
	}

	pub fn groups_from_image_size(&mut self, image: impl Into<ImageArgument>) -> &mut Self {
		self.cmd.dispatch_size = DispatchSize::DeriveFromImage(image.into());
		self
	}

	pub fn buffer(&mut self, target: impl Into<BufferBindTarget>, buffer: impl IntoBufferArgument) -> &mut Self {
		self.cmd.bindings.bind_buffer(target, buffer.into_buffer_argument(self.upload_stage));
		self
	}

	pub fn ubo(&mut self, index: u32, buffer: impl IntoBufferArgument) -> &mut Self {
		self.buffer(BufferBindTarget::UboIndex(index), buffer)
	}

	pub fn ssbo(&mut self, index: u32, buffer: impl IntoBufferArgument) -> &mut Self {
		self.buffer(BufferBindTarget::SsboIndex(index), buffer)
	}

	pub fn sampled_image(&mut self, unit: u32, image: impl Into<ImageArgument>, sampler: impl Into<SamplerArgument>) -> &mut Self {
		self.cmd.bindings.bind_sampled_image(ImageBindTarget::Sampled(unit), image, sampler);
		self
	}

	pub fn image(&mut self, unit: u32, image: impl Into<ImageArgument>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image);
		self
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn image_rw(&mut self, unit: u32, image: impl Into<ImageArgument>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image);
		self
	}
}

