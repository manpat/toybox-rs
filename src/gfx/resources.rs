use crate::prelude::*;
use crate::gfx::*;

use crate::utility::resource::{Resource, ResourceStore, ResourceLock, ResourceLockMut};


slotmap::new_key_type!{
	pub struct TextureKey;
	pub struct FramebufferKey;
}



#[derive(Debug)]
pub struct Resources {
	pub textures: ResourceStore<Texture>,
	pub framebuffers: ResourceStore<Framebuffer>,
}

impl Resources {
	pub fn get<H>(&self, handle: H) -> ResourceLock<H::Resource>
		where H: ResourceKey
	{
		handle.get(self)
	}

	pub fn get_mut<H>(&mut self, handle: H) -> ResourceLockMut<H::Resource>
		where H: ResourceKey
	{
		handle.get_mut(self)
	}
}

impl Resources {
	pub(super) fn new() -> Resources {
		Resources {
			textures: ResourceStore::new(),
			framebuffers: ResourceStore::new(),
		}
	}

	pub(super) fn on_backbuffer_resize(&mut self, backbuffer_size: Vec2i) {
		self.textures.foreach_mut(|texture| {
			texture.on_backbuffer_resize(backbuffer_size);
		});

		self.framebuffers.foreach_mut(|framebuffer| {
			framebuffer.rebind_attachments(&self.textures);
		});
	}
}


pub trait ResourceKey {
	type Resource : Resource;

	fn get(&self, _: &Resources) -> ResourceLock<Self::Resource>;
	fn get_mut(&self, _: &mut Resources) -> ResourceLockMut<Self::Resource>;
}


impl ResourceKey for TextureKey {
	type Resource = Texture;

	fn get(&self, resources: &Resources) -> ResourceLock<Self::Resource> {
		resources.textures.get(*self)
	}

	fn get_mut(&self, resources: &mut Resources) -> ResourceLockMut<Self::Resource> {
		resources.textures.get_mut(*self)
	}
}


impl ResourceKey for FramebufferKey {
	type Resource = Framebuffer;

	fn get(&self, resources: &Resources) -> ResourceLock<Self::Resource> {
		resources.framebuffers.get(*self)
	}

	fn get_mut(&self, resources: &mut Resources) -> ResourceLockMut<Self::Resource> {
		resources.framebuffers.get_mut(*self)
	}
}


impl Resource for Texture {
	type Key = TextureKey;
}

impl Resource for Framebuffer {
	type Key = FramebufferKey;
}



pub trait IntoTextureKey {
	fn into_texture_key(self, resources: &Resources) -> TextureKey;
}

impl IntoTextureKey for TextureKey {
	fn into_texture_key(self, _: &Resources) -> TextureKey { self }
}

impl IntoTextureKey for FramebufferColorAttachmentKey {
	fn into_texture_key(self, resources: &Resources) -> TextureKey {
		let fb = resources.framebuffers.get(self.0);
		fb.color_attachment(self.1)
			.unwrap()
	}
}

impl IntoTextureKey for FramebufferDepthStencilAttachmentKey {
	fn into_texture_key(self, resources: &Resources) -> TextureKey {
		let fb = resources.framebuffers.get(self.0);
		fb.depth_stencil_attachment().unwrap()
	}
}



impl FramebufferKey {
	pub fn color_attachment(self, attachment: u32) -> FramebufferColorAttachmentKey {
		FramebufferColorAttachmentKey(self, attachment)
	}

	pub fn depth_stencil_attachment(self) -> FramebufferDepthStencilAttachmentKey {
		FramebufferDepthStencilAttachmentKey(self)
	}
}


#[derive(Copy, Clone, Debug)]
pub struct FramebufferColorAttachmentKey(FramebufferKey, u32);

#[derive(Copy, Clone, Debug)]
pub struct FramebufferDepthStencilAttachmentKey(FramebufferKey);



impl ResourceKey for FramebufferColorAttachmentKey {
	type Resource = Texture;

	fn get(&self, resources: &Resources) -> ResourceLock<Self::Resource> {
		let texture_key = self.into_texture_key(resources);
		resources.textures.get(texture_key)
	}

	fn get_mut(&self, resources: &mut Resources) -> ResourceLockMut<Self::Resource> {
		let texture_key = self.into_texture_key(resources);
		resources.textures.get_mut(texture_key)
	}
}

impl ResourceKey for FramebufferDepthStencilAttachmentKey {
	type Resource = Texture;

	fn get(&self, resources: &Resources) -> ResourceLock<Self::Resource> {
		let texture_key = self.into_texture_key(resources);
		resources.textures.get(texture_key)
	}

	fn get_mut(&self, resources: &mut Resources) -> ResourceLockMut<Self::Resource> {
		let texture_key = self.into_texture_key(resources);
		resources.textures.get_mut(texture_key)
	}
}