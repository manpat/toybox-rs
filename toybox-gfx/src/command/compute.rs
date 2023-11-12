use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTarget, BufferBindSource, IntoBufferBindSourceOrStageable, ImageBindSource, ImageBindTarget};
use crate::resource_manager::ShaderHandle;
use crate::upload_heap::UploadStage;
use crate::core::SamplerName;

#[derive(Debug)]
pub enum DispatchSize {
	Explicit(Vec3i),
	Indirect(BufferBindSource),
}


#[derive(Debug)]
pub struct ComputeCmd {
	pub compute_shader: ShaderHandle,
	pub dispatch_size: DispatchSize,
	pub bindings: BindingDescription,
}

impl From<ComputeCmd> for super::Command {
	fn from(o: ComputeCmd) -> Self {
		Self::Compute(o)
	}
}

impl ComputeCmd {
	pub fn new(compute_shader: ShaderHandle) -> ComputeCmd {
		ComputeCmd {
			compute_shader,
			dispatch_size: DispatchSize::Explicit(Vec3i::splat(1)),
			bindings: Default::default(),
		}
	}

	pub fn execute(&self, core: &mut crate::core::Core, rm: &mut crate::resource_manager::ResourceManager) {
		let pipeline = rm.resolve_compute_pipeline(core, self.compute_shader);
		core.bind_shader_pipeline(pipeline);

		self.bindings.bind(core, rm);

		let mut barrier_tracker = core.barrier_tracker();

		match self.dispatch_size {
			DispatchSize::Explicit(size) => unsafe {
				barrier_tracker.emit_barriers(&core.gl);
				core.gl.DispatchCompute(size.x as u32, size.y as u32, size.z as u32);
			}

			DispatchSize::Indirect(bind_source) => {
				let BufferBindSource::Name{name, range} = bind_source
					else { panic!("Unresolved buffer bind source description") };

				let offset = range.map(|r| r.offset).unwrap_or(0);

				core.bind_dispatch_indirect_buffer(name);

				barrier_tracker.read_buffer(name, gl::COMMAND_BARRIER_BIT);
				barrier_tracker.emit_barriers(&core.gl);

				unsafe {
					core.gl.DispatchComputeIndirect(offset as isize);
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

	pub fn indirect(&mut self, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.dispatch_size = DispatchSize::Indirect(buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn buffer(&mut self, target: impl Into<BufferBindTarget>, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.bindings.bind_buffer(target, buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ubo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.buffer(BufferBindTarget::UboIndex(index), buffer)
	}

	pub fn ssbo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.buffer(BufferBindTarget::SsboIndex(index), buffer)
	}

	pub fn sampled_image(&mut self, unit: u32, image: impl Into<ImageBindSource>, sampler: SamplerName) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::Sampled(unit), image, sampler);
		self
	}

	pub fn image(&mut self, unit: u32, image: impl Into<ImageBindSource>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image, None);
		self
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn image_rw(&mut self, unit: u32, image: impl Into<ImageBindSource>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image, None);
		self
	}
}

