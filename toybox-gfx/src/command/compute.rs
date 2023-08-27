use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTargetDesc, BufferBindSourceDesc, IntoBufferBindSourceOrStageable};
use crate::resource_manager::shader::ShaderHandle;
use crate::upload_heap::UploadStage;

#[derive(Debug)]
pub enum DispatchSize {
	Explicit(Vec3i),
	Indirect(BufferBindSourceDesc),
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

		self.bindings.bind(core);

		match self.dispatch_size {
			DispatchSize::Explicit(size) => unsafe {
				core.gl.DispatchCompute(size.x as u32, size.y as u32, size.z as u32);
			}

			DispatchSize::Indirect(bind_source) => {
				let BufferBindSourceDesc::Name{name, range} = bind_source
					else { panic!("Unresolved buffer bind source description") };

				let offset = range.map(|r| r.offset).unwrap_or(0);

				core.bind_draw_indirect_buffer(name);

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

	// pub fn indexed(&mut self, buffer: impl IntoBufferHandle) -> &mut Self {
	// 	let buffer_handle = buffer.into_buffer_handle(self.frame_state);
	// 	self.cmd.index_buffer = Some(buffer_handle);
	// 	self
	// }

	pub fn buffer(&mut self, target: impl Into<BufferBindTargetDesc>, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.bindings.bind_buffer(target, buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ubo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.bindings.bind_buffer(BufferBindTargetDesc::UboIndex(index), buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ssbo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.bindings.bind_buffer(BufferBindTargetDesc::SsboIndex(index), buffer.into_bind_source(self.upload_stage));
		self
	}

	// pub fn texture(&mut self, location: impl Into<ImageBindingLocation>, image: ImageHandle, sampler: SamplerDef) -> &mut Self {
	// 	self.cmd.image_bindings.push(ImageBinding::texture(image, sampler, location));
	// 	self
	// }

	// pub fn image(&mut self, location: impl Into<ImageBindingLocation>, image: ImageHandle) -> &mut Self {
	// 	self.cmd.image_bindings.push(ImageBinding::image(image, location));
	// 	self
	// }

	// pub fn image_rw(&mut self, location: impl Into<ImageBindingLocation>, image: ImageHandle) -> &mut Self {
	// 	self.cmd.image_bindings.push(ImageBinding::image_rw(image, location));
	// 	self
	// }
}

