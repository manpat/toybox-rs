use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTarget, BufferBindSource, IntoBufferBindSourceOrStageable, ImageBindSource, ImageBindTarget};
use crate::resource_manager::ShaderHandle;
use crate::upload_heap::UploadStage;
use crate::core::SamplerName;


#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum PrimitiveType {
	Points = gl::POINTS,
	Lines = gl::LINES,
	Triangles = gl::TRIANGLES,
}


#[derive(Debug)]
pub struct DrawCmd {
	pub bindings: BindingDescription,

	pub vertex_shader: ShaderHandle,
	pub fragment_shader: Option<ShaderHandle>,

	pub primitive_type: PrimitiveType,

	pub num_elements: u32,
	pub num_instances: u32,

	pub index_buffer: Option<BufferBindSource>,
}

impl From<DrawCmd> for super::Command {
	fn from(o: DrawCmd) -> Self {
		Self::Draw(o)
	}
}

impl DrawCmd {
	pub fn from_shaders(vertex_shader: ShaderHandle, fragment_shader: impl Into<Option<ShaderHandle>>) -> DrawCmd {
		let fragment_shader = fragment_shader.into();

		DrawCmd {
			vertex_shader,
			fragment_shader,
			primitive_type: PrimitiveType::Triangles,

			num_elements: 3,
			num_instances: 1,

			index_buffer: None,

			bindings: Default::default(),
		}
	}

	pub fn execute(&self, core: &mut crate::core::Core, rm: &mut crate::resource_manager::ResourceManager) {
		// TODO(pat.m): eugh. should probably be part of a larger pipeline state management system
		let num_user_clip_planes = rm.shaders.get_resource(self.vertex_shader).unwrap().num_user_clip_planes;
		core.set_user_clip_planes(num_user_clip_planes);

		let pipeline = rm.resolve_draw_pipeline(core, self.vertex_shader, self.fragment_shader);
		core.bind_shader_pipeline(pipeline);

		self.bindings.bind(core);

		let primitive_type = self.primitive_type as u32;
		let num_elements = self.num_elements as i32;
		let num_instances = self.num_instances as i32;

		let mut barrier_tracker = core.barrier_tracker();

		if let Some(bind_source) = self.index_buffer {
			let BufferBindSource::Name{name, range} = bind_source
				else { panic!("Unresolved buffer bind source description") };

			// TODO(pat.m): allow non 32b indices
			let index_type = gl::UNSIGNED_INT;
			let offset_ptr = range.map(|r| r.offset).unwrap_or(0) as *const _;

			core.bind_index_buffer(name);

			barrier_tracker.read_buffer(name, gl::ELEMENT_ARRAY_BARRIER_BIT);
			barrier_tracker.emit_barriers(&core.gl);

			unsafe {
				core.gl.DrawElementsInstanced(primitive_type, num_elements, index_type,
					offset_ptr, num_instances);
			}

		} else {
			barrier_tracker.emit_barriers(&core.gl);

			unsafe {
				core.gl.DrawArraysInstanced(primitive_type, 0, num_elements, num_instances);
			}
		}
	}
}


pub struct DrawCmdBuilder<'cg> {
	pub(crate) cmd: &'cg mut DrawCmd,
	pub(crate) upload_stage: &'cg mut UploadStage,
}

impl<'cg> DrawCmdBuilder<'cg> {
	pub fn elements(&mut self, num_elements: u32) -> &mut Self {
		self.cmd.num_elements = num_elements;
		self
	}

	pub fn instances(&mut self, num_instances: u32) -> &mut Self {
		self.cmd.num_instances = num_instances;
		self
	}

	pub fn primitive(&mut self, ty: PrimitiveType) -> &mut Self {
		self.cmd.primitive_type = ty;
		self
	}

	pub fn indexed(&mut self, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		let bind_source = buffer.into_bind_source(self.upload_stage);
		self.cmd.index_buffer = Some(bind_source);
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

