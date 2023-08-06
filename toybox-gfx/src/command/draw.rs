use crate::prelude::*;
use crate::bindings::{BindingDescription, BufferBindTargetDesc, BufferBindSourceDesc};
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
pub struct DrawArgs {
	pub vertex_shader: ShaderHandle,
	pub fragment_shader: Option<ShaderHandle>,

	pub primitive_type: PrimitiveType,

	pub num_elements: u32,
	pub num_instances: u32,

	// pub index_buffer: Option<BufferHandle>,
}



#[derive(Debug)]
pub struct DrawCmd {
	pub args: DrawArgs,
	pub bindings: BindingDescription,
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
			args: DrawArgs {
				vertex_shader,
				fragment_shader,
				primitive_type: PrimitiveType::Triangles,

				num_elements: 3,
				num_instances: 1,
			},

			bindings: Default::default(),
		}
	}

	pub fn execute(&self, core: &mut crate::core::Core, rm: &mut crate::resource_manager::ResourceManager) {
		let pipeline = rm.resolve_draw_pipeline(core, self.args.vertex_shader, self.args.fragment_shader);
		core.bind_shader_pipeline(pipeline);
		core.bind_vao(rm.global_vao);

		self.bindings.bind(core);

		let primitive_type = self.args.primitive_type as u32;

		unsafe {
			core.gl.DrawArraysInstanced(primitive_type, 0, self.args.num_elements as i32, self.args.num_instances as i32);
		}
	}
}


pub struct DrawCmdBuilder<'cg> {
	pub(crate) cmd: &'cg mut DrawCmd,
	pub(crate) upload_stage: &'cg mut UploadStage,
}

impl<'cg> DrawCmdBuilder<'cg> {
	pub fn elements(&mut self, num_elements: u32) -> &mut Self {
		self.cmd.args.num_elements = num_elements;
		self
	}

	pub fn instances(&mut self, num_instances: u32) -> &mut Self {
		self.cmd.args.num_instances = num_instances;
		self
	}

	pub fn primitive(&mut self, ty: PrimitiveType) -> &mut Self {
		self.cmd.args.primitive_type = ty;
		self
	}

	// pub fn indexed(&mut self, buffer: impl IntoBufferHandle) -> &mut Self {
	// 	let buffer_handle = buffer.into_buffer_handle(self.frame_state);
	// 	self.cmd.index_buffer = Some(buffer_handle);
	// 	self
	// }

	pub fn buffer(&mut self, target: impl Into<BufferBindTargetDesc>, buffer: impl IntoBufferBindSourceOrData) -> &mut Self {
		self.cmd.bindings.bind_buffer(target, buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ubo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrData) -> &mut Self {
		self.cmd.bindings.bind_buffer(BufferBindTargetDesc::UboIndex(index), buffer.into_bind_source(self.upload_stage));
		self
	}

	pub fn ssbo(&mut self, index: u32, buffer: impl IntoBufferBindSourceOrData) -> &mut Self {
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



pub trait IntoBufferBindSourceOrData {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSourceDesc;
}

impl<T> IntoBufferBindSourceOrData for T
	where T: Into<BufferBindSourceDesc>
{
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSourceDesc {
		self.into()
	}
}