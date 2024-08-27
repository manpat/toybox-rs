use crate::prelude::*;
use crate::bindings::*;

use crate::{
	Core, ResourceManager,
	ShaderArgument,
	BlendMode,
	upload_heap::UploadStage,
	arguments::*,
};


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

	vertex_shader: ShaderArgument,
	fragment_shader: Option<ShaderArgument>,

	pub primitive_type: PrimitiveType,

	pub num_elements: u32,
	pub num_instances: u32,

	pub index_buffer: Option<BufferArgument>,

	// TODO(pat.m): different name?
	pub base_vertex: u32,

	pub blend_mode: Option<BlendMode>,
	pub depth_test: bool,
	pub depth_write: bool,
}

impl From<DrawCmd> for super::Command {
	fn from(o: DrawCmd) -> Self {
		Self::Draw(o)
	}
}

impl DrawCmd {
	pub fn from_shaders(vertex_shader: ShaderArgument, fragment_shader: Option<ShaderArgument>) -> DrawCmd {
		DrawCmd {
			bindings: Default::default(),

			vertex_shader,
			fragment_shader: fragment_shader,

			primitive_type: PrimitiveType::Triangles,

			num_elements: 3,
			num_instances: 1,

			index_buffer: None,
			base_vertex: 0,

			blend_mode: None,
			depth_test: true,
			depth_write: true,
		}
	}

	pub fn from_fullscreen_shader(fragment_shader: ShaderArgument) -> DrawCmd {
		DrawCmd {
			bindings: Default::default(),

			vertex_shader: CommonShader::FullscreenVertex.into(),
			fragment_shader: Some(fragment_shader),

			primitive_type: PrimitiveType::Triangles,

			num_elements: 6,
			num_instances: 1,

			index_buffer: None,
			base_vertex: 0,

			blend_mode: None,
			depth_test: false,
			depth_write: false,
		}
	}

	pub fn execute(&self, core: &mut Core, rm: &mut ResourceManager) {
		let vertex_shader_handle = match self.vertex_shader {
			ShaderArgument::Handle(name) => name,
			ShaderArgument::Common(shader) => rm.get_common_shader(shader),
		};

		let fragment_shader_handle = match self.fragment_shader {
			Some(ShaderArgument::Handle(name)) => Some(name),
			Some(ShaderArgument::Common(shader)) => Some(rm.get_common_shader(shader)),
			None => None,
		};

		// TODO(pat.m): eugh. should probably be part of a larger pipeline state management system
		let num_user_clip_planes = rm.shaders.get_resource(vertex_shader_handle).unwrap().num_user_clip_planes;
		core.set_user_clip_planes(num_user_clip_planes);

		let pipeline = rm.resolve_draw_pipeline(core, vertex_shader_handle, fragment_shader_handle);
		core.bind_shader_pipeline(pipeline);

		core.set_blend_mode(self.blend_mode);
		core.set_depth_test(self.depth_test);
		core.set_depth_write(self.depth_write);

		self.bindings.bind(core, rm);

		let primitive_type = self.primitive_type as u32;
		let num_elements = self.num_elements as i32;
		let num_instances = self.num_instances as i32;

		let mut barrier_tracker = core.barrier_tracker();

		if let Some(buffer_argument) = self.index_buffer {
			let BufferArgument::Name{name, range} = buffer_argument
				else { panic!("Unresolved buffer bind source description") };

			// TODO(pat.m): allow non 32b indices
			let index_type = gl::UNSIGNED_INT;
			let offset_ptr = range.map_or(0, |r| r.offset) as *const _;
			let base_vertex = self.base_vertex as i32;

			core.bind_index_buffer(name);

			barrier_tracker.read_buffer(name, gl::ELEMENT_ARRAY_BARRIER_BIT);
			barrier_tracker.emit_barriers(&core.gl);

			unsafe {
				core.gl.DrawElementsInstancedBaseVertex(primitive_type, num_elements, index_type,
					offset_ptr, num_instances, base_vertex);
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

	pub fn indexed(&mut self, buffer: impl IntoBufferArgument) -> &mut Self {
		let buffer_argument = buffer.into_buffer_argument(self.upload_stage);
		self.cmd.index_buffer = Some(buffer_argument);
		self
	}

	pub fn base_vertex(&mut self, base_vertex: u32) -> &mut Self {
		self.cmd.base_vertex = base_vertex;
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

	pub fn rendertargets(&mut self, rts: impl Into<FramebufferArgument>) -> &mut Self {
		self.cmd.bindings.bind_framebuffer(rts);
		self
	}

	pub fn blend_mode(&mut self, blend_mode: impl Into<Option<BlendMode>>) -> &mut Self {
		self.cmd.blend_mode = blend_mode.into();
		self
	}

	pub fn depth_test(&mut self, depth_test: bool) -> &mut Self {
		self.cmd.depth_test = depth_test;
		self
	}

	pub fn depth_write(&mut self, depth_write: bool) -> &mut Self {
		self.cmd.depth_write = depth_write;
		self
	}
}
