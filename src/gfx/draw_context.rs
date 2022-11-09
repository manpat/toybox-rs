use crate::prelude::*;
use crate::gfx::*;
use crate::utility::{ResourceLock};


/// Provides access to everything needed to set up and submit draw calls and dispatch compute shaders.
///
/// Existence of this object prohibits creation of new resources and general modification of [`System`]
/// until in-progress draw calls are submitted.
///
/// ## Note
/// Dispatching compute shaders is still fairly raw, so it may occasionally be necessary to dip into [`raw`]
/// api calls to set things up manually. The main thing this is needed for currently is for submitting [`glMemoryBarrier`](raw::MemoryBarrier())
/// calls, which I haven't figured out a good api for yet.
pub struct DrawContext<'ctx> {
	pub(super) resources: &'ctx Resources,
	pub(super) backbuffer_size: Vec2i,
}

impl<'ctx> DrawContext<'ctx> {
	pub fn set_wireframe(&mut self, wireframe_enabled: bool) {
		let mode = match wireframe_enabled {
			false => raw::FILL,
			true => raw::LINE,
		};

		unsafe {
			raw::PolygonMode(raw::FRONT_AND_BACK, mode);
		}
	}

	pub fn set_backface_culling(&mut self, culling_enabled: bool) {
		unsafe {
			match culling_enabled {
				true => raw::Enable(raw::CULL_FACE),
				false => raw::Disable(raw::CULL_FACE),
			}
		}
	}

	pub fn set_clear_color(&mut self, color: impl Into<Color>) {
		let (r,g,b,a) = color.into().to_tuple();
		unsafe {
			raw::ClearColor(r, g, b, a);
		}
	}

	pub fn clear(&mut self, mode: ClearMode) {
		unsafe {
			raw::Clear(mode.into_gl());
		}
	}

	pub fn resources(&self) -> &'ctx Resources {
		self.resources
	}

	pub fn get_framebuffer(&self, framebuffer: FramebufferKey) -> ResourceLock<Framebuffer> {
		self.resources.get(framebuffer)
	}

	pub fn get_texture(&self, texture: TextureKey) -> ResourceLock<Texture> {
		self.resources.get(texture)
	}

	pub fn bind_uniform_buffer<T: Copy>(&mut self, binding: u32, buffer: Buffer<T>) {
		unsafe {
			raw::BindBufferBase(raw::UNIFORM_BUFFER, binding, buffer.handle);
		}
	}

	pub fn bind_shader_storage_buffer<T: Copy>(&mut self, binding: u32, buffer: Buffer<T>) {
		unsafe {
			raw::BindBufferBase(raw::SHADER_STORAGE_BUFFER, binding, buffer.handle);
		}
	}

	fn bind_image_raw(&mut self, binding: u32, texture: TextureKey, rw_flags: u32) {
		// https://www.khronos.org/opengl/wiki/Image_Load_Store#Images_in_the_context
		let (level, layered, layer) = (0, 0, 0);
		let texture = texture.get(self.resources);

		unsafe {
			raw::BindImageTexture(binding, texture.texture_handle, level, layered, layer,
				rw_flags, texture.format().to_gl());
		}
	}

	pub fn bind_image_for_rw(&mut self, binding: u32, texture: TextureKey) {
		self.bind_image_raw(binding, texture, raw::READ_WRITE)
	}

	pub fn bind_image_for_read(&mut self, binding: u32, texture: TextureKey) {
		self.bind_image_raw(binding, texture, raw::READ_ONLY)
	}

	pub fn bind_image_for_write(&mut self, binding: u32, texture: TextureKey) {
		self.bind_image_raw(binding, texture, raw::WRITE_ONLY)
	}

	pub fn bind_texture(&mut self, binding: u32, texture: TextureKey) {
		let texture = texture.get(self.resources);

		unsafe {
			raw::BindTextureUnit(binding, texture.texture_handle);
			raw::BindSampler(binding, texture.sampler_handle);
		}
	}

	pub fn bind_vao(&mut self, vao: Vao) {
		unsafe {
			raw::BindVertexArray(vao.handle);
		}
	}

	pub fn bind_shader(&mut self, shader: Shader) {
		unsafe {
			raw::UseProgram(shader.0);
		}
	}

	pub fn bind_framebuffer(&mut self, framebuffer: impl Into<Option<FramebufferKey>>) {
		if let Some(framebuffer) = framebuffer.into() {
			let framebuffer = framebuffer.get(self.resources);
			let Vec2i{x,y} = framebuffer.size_mode.resolve(self.backbuffer_size);

			unsafe {
				raw::Viewport(0, 0, x, y);
				raw::BindFramebuffer(raw::DRAW_FRAMEBUFFER, framebuffer.handle);
			}
		} else {
			let Vec2i{x,y} = self.backbuffer_size;

			unsafe {
				raw::Viewport(0, 0, x, y);
				raw::BindFramebuffer(raw::DRAW_FRAMEBUFFER, 0);
			}
		}
	}

	/// Call before drawcalls that read textures/images written by shaders in previous drawcalls.
	pub fn insert_texture_barrier(&self) {
		unsafe {
			raw::MemoryBarrier(raw::TEXTURE_FETCH_BARRIER_BIT | raw::SHADER_IMAGE_ACCESS_BARRIER_BIT);
		}
	}

	/// Call before drawcalls that read shader storage buffers written by shaders in previous drawcalls.
	pub fn insert_shader_storage_barrier(&self) {
		unsafe {
			raw::MemoryBarrier(raw::SHADER_STORAGE_BARRIER_BIT);
		}
	}

	/// Call before drawcalls that read uniform buffers written by shaders in previous drawcalls.
	pub fn insert_uniform_barrier(&self) {
		unsafe {
			raw::MemoryBarrier(raw::UNIFORM_BARRIER_BIT);
		}
	}

	pub fn draw_arrays(&self, draw_mode: DrawMode, num_vertices: u32) {
		if num_vertices == 0 {
			return
		}

		unsafe {
			raw::DrawArrays(draw_mode.into_gl(), 0, num_vertices as i32);
		}
	}

	pub fn draw_indexed(&self, draw_mode: DrawMode, element_range: impl Into<IndexedDrawParams>) {
		let IndexedDrawParams {
			num_elements,
			element_offset,
			base_vertex,
		} = element_range.into();

		if num_elements == 0 {
			return
		}

		let offset_ptr = (element_offset as usize * std::mem::size_of::<u16>()) as *const _;

		unsafe {
			raw::DrawElementsBaseVertex(draw_mode.into_gl(), num_elements as i32, raw::UNSIGNED_SHORT, offset_ptr, base_vertex as i32);
		}
	}

	pub fn draw_instances_indexed(&self, draw_mode: DrawMode, num_elements: u32, num_instances: u32) {
		if num_elements == 0 || num_instances == 0 {
			return
		}

		unsafe {
			raw::DrawElementsInstanced(draw_mode.into_gl(), num_elements as i32, raw::UNSIGNED_SHORT, std::ptr::null(), num_instances as i32);
		}
	}

	pub fn dispatch_compute(&self, x: u32, y: u32, z: u32) {
		// see: GL_MAX_COMPUTE_WORK_GROUP_COUNT
		assert!(x < 65536, "Work group exceeds guaranteed minimum size along x axis");
		assert!(y < 65536, "Work group exceeds guaranteed minimum size along y axis");
		assert!(z < 65536, "Work group exceeds guaranteed minimum size along z axis");

		unsafe {
			raw::DispatchCompute(x, y, z);
		}
	}
}