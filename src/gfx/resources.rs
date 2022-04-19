use crate::prelude::*;
use crate::gfx::*;

use crate::utility::resource::{Resource, ResourceStore, ResourceLock, ResourceLockMut};


slotmap::new_key_type!{
	pub struct TextureKey;
	pub struct FramebufferKey;
}



#[derive(Debug)]
pub struct Resources {
	textures: ResourceStore<Texture>,
	framebuffers: ResourceStore<Framebuffer>,
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

	pub(super) fn insert_texture(&mut self, texture: Texture) -> TextureKey {
		self.textures.insert(texture)
	}

	pub(super) fn insert_framebuffer(&mut self, framebuffer: Framebuffer) -> FramebufferKey {
		self.framebuffers.insert(framebuffer)
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


pub trait ResourceKey : slotmap::Key {
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