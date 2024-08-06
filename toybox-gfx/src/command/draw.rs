use crate::prelude::*;
use crate::bindings::*;
use crate::resource_manager::{ShaderHandle};
use crate::upload_heap::UploadStage;
use crate::core::*;


#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum PrimitiveType {
	Points = gl::POINTS,
	Lines = gl::LINES,
	Triangles = gl::TRIANGLES,
}

#[derive(Debug, Copy, Clone)]
enum DrawCmdShaders {
	Pair { vertex_shader: ShaderHandle, fragment_shader: Option<ShaderHandle> },
	FullscreenQuad { fragment_shader: Option<ShaderHandle> },
}


#[derive(Debug)]
pub struct DrawCmd {
	pub bindings: BindingDescription,

	shaders: DrawCmdShaders,

	pub primitive_type: PrimitiveType,

	pub num_elements: u32,
	pub num_instances: u32,

	pub index_buffer: Option<BufferBindSource>,

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
	pub fn from_shaders(vertex_shader: ShaderHandle, fragment_shader: impl Into<Option<ShaderHandle>>) -> DrawCmd {
		let fragment_shader = fragment_shader.into();

		DrawCmd {
			bindings: Default::default(),

			shaders: DrawCmdShaders::Pair {
				vertex_shader,
				fragment_shader,
			},

			primitive_type: PrimitiveType::Triangles,

			num_elements: 3,
			num_instances: 1,

			index_buffer: None,

			blend_mode: None,
			depth_test: true,
			depth_write: true,
		}
	}

	pub fn from_fullscreen_shader(fragment_shader: impl Into<Option<ShaderHandle>>) -> DrawCmd {
		let fragment_shader = fragment_shader.into();

		DrawCmd {
			bindings: Default::default(),

			shaders: DrawCmdShaders::FullscreenQuad{fragment_shader},
			primitive_type: PrimitiveType::Triangles,

			num_elements: 6,
			num_instances: 1,

			index_buffer: None,

			blend_mode: None,
			depth_test: false,
			depth_write: false,
		}
	}

	pub fn execute(&self, core: &mut crate::core::Core, rm: &mut crate::resource_manager::ResourceManager) {
		let (vs, fs) = match self.shaders {
			DrawCmdShaders::Pair {vertex_shader, fragment_shader} => (vertex_shader, fragment_shader),
			DrawCmdShaders::FullscreenQuad {fragment_shader}
				=> (rm.fullscreen_vs_shader, Some(fragment_shader.unwrap_or(rm.flat_fs_shader))),
		};

		// TODO(pat.m): eugh. should probably be part of a larger pipeline state management system
		let num_user_clip_planes = rm.shaders.get_resource(vs).unwrap().num_user_clip_planes;
		core.set_user_clip_planes(num_user_clip_planes);

		let pipeline = rm.resolve_draw_pipeline(core, vs, fs);
		core.bind_shader_pipeline(pipeline);

		core.set_blend_mode(self.blend_mode);
		core.set_depth_test(self.depth_test);
		core.set_depth_write(self.depth_write);

		self.bindings.bind(core, rm);

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

			let base_vertex = 0;

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

	pub fn sampled_image(&mut self, unit: u32, image: impl Into<ImageNameOrHandle>, sampler: SamplerName) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::Sampled(unit), image, sampler);
		self
	}

	pub fn image(&mut self, unit: u32, image: impl Into<ImageNameOrHandle>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadonlyImage(unit), image, None);
		self
	}

	// TODO(pat.m): do I want RW to be explicit?
	pub fn image_rw(&mut self, unit: u32, image: impl Into<ImageNameOrHandle>) -> &mut Self {
		self.cmd.bindings.bind_image(ImageBindTarget::ReadWriteImage(unit), image, None);
		self
	}

	pub fn rendertargets(&mut self, rts: impl Into<FramebufferDescriptionOrName>) -> &mut Self {
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

