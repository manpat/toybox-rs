use crate::prelude::*;
use crate::gfx::*;

use resource_scope::{ResourceScope, ScopedResourceHandle};


/// Allows creation of new resources within a `ResourceScope` (which may be the global `ResourceScope`).
/// Existence of this object prohibits submitting draw calls and modifying pipeline state, plus general modification of [`System`].
pub struct ResourceContext<'ctx> {
	pub resources: &'ctx mut Resources,

	pub(super) shader_manager: &'ctx mut ShaderManager,
	pub(super) resource_scope: &'ctx mut ResourceScope,
	pub(super) capabilities: &'ctx Capabilities,
	pub(super) backbuffer_size: Vec2i,
}

/// Reexposes utilities on [`System`] for convenience.
impl<'ctx> ResourceContext<'ctx> {
	pub fn backbuffer_size(&self) -> Vec2i { self.backbuffer_size }
	pub fn aspect(&self) -> f32 {
		let Vec2{x, y} = self.backbuffer_size.to_vec2();
		x / y
	}

	pub fn capabilities(&self) -> &Capabilities { self.capabilities }
}

/// Resource creation.
impl<'ctx> ResourceContext<'ctx> {
	pub fn new_buffer<T: Copy>(&mut self, usage: BufferUsage) -> Buffer<T> {
		let mut handle = 0;
		unsafe {
			raw::CreateBuffers(1, &mut handle);
		}

		self.resource_scope.insert(ScopedResourceHandle::Buffer{handle});
		Buffer::from_raw(handle, usage)
	}

	pub fn new_texture(&mut self, size: impl Into<TextureSize>, format: TextureFormat) -> TextureKey {
		let texture = Texture::new(size.into(), self.backbuffer_size, format);
		let key = self.resources.textures.insert(texture);
		self.resource_scope.insert(ScopedResourceHandle::Texture{key});
		key
	}

	pub fn new_framebuffer(&mut self, settings: FramebufferSettings) -> FramebufferKey {
		let framebuffer = Framebuffer::new(settings, &mut self.resources, self.backbuffer_size);
		let key = self.resources.framebuffers.insert(framebuffer);
		self.resource_scope.insert(ScopedResourceHandle::Framebuffer{key});
		key
	}

	pub fn new_vao(&mut self) -> Vao {
		unsafe {
			let mut handle = 0;
			raw::CreateVertexArrays(1, &mut handle);
			self.resource_scope.insert(ScopedResourceHandle::Vao{handle});
			Vao::new(handle)
		}
	}

	pub fn new_query(&mut self) -> QueryObject {
		unsafe {
			let mut handle = 0;
			raw::GenQueries(1, &mut handle);
			self.resource_scope.insert(ScopedResourceHandle::Query{handle});
			QueryObject(handle)
		}
	}


	pub fn add_shader_import(&mut self, name: impl Into<String>, src: impl Into<String>) {
		self.shader_manager.add_import(name, src)
	}

	pub fn new_shader(&mut self, shaders: &[(u32, &str)]) -> Result<Shader, shader::CompilationError> {
		self.shader_manager.get_shader(shaders)
	}

	pub fn new_simple_shader(&mut self, vsrc: &str, fsrc: &str) -> Result<Shader, shader::CompilationError> {
		self.shader_manager.get_shader(&[
			(raw::VERTEX_SHADER, vsrc),
			(raw::FRAGMENT_SHADER, fsrc),
		])
	}

	pub fn new_compute_shader(&mut self, csrc: &str) -> Result<Shader, shader::CompilationError> {
		self.shader_manager.get_shader(&[
			(raw::COMPUTE_SHADER, csrc)
		])
	}
}

