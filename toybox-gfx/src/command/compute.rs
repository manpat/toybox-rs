use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTargetDesc, BufferBindSourceDesc};
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