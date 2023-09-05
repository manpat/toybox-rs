use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTargetDesc, BufferBindSourceDesc, IntoBufferBindSourceOrStageable};
use crate::resource_manager::shader::ShaderHandle;
use crate::upload_heap::UploadStage;


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

	pub index_buffer: Option<BufferBindSourceDesc>,
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
		let pipeline = rm.resolve_draw_pipeline(core, self.vertex_shader, self.fragment_shader);
		core.bind_shader_pipeline(pipeline);
		core.bind_vao(rm.global_vao);

		self.bindings.bind(core);

		let primitive_type = self.primitive_type as u32;
		let num_elements = self.num_elements as i32;
		let num_instances = self.num_instances as i32;

		if let Some(bind_source) = self.index_buffer {
			let BufferBindSourceDesc::Name{name, range} = bind_source
				else { panic!("Unresolved buffer bind source description") };

			// TODO(pat.m): allow non 32b indices
			let index_type = gl::UNSIGNED_INT;
			let offset_ptr = range.map(|r| r.offset).unwrap_or(0) as *const _;

			core.set_vao_index_buffer(rm.global_vao, name);

			unsafe {
				core.gl.DrawElementsInstanced(primitive_type, num_elements, index_type,
					offset_ptr, num_instances);
			}

		} else {
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

	pub fn buffer(&mut self, target: impl Into<BufferBindTargetDesc>, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.cmd.bindings.bind_buffer(target, buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ubo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.buffer(BufferBindTargetDesc::UboIndex(index), buffer)
	}

	pub fn ssbo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrStageable) -> &mut Self {
		self.buffer(BufferBindTargetDesc::SsboIndex(index), buffer)
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

