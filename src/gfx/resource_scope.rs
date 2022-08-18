use crate::gfx::*;
use std::rc::Rc;


/// Unique identifier for a `ResourceScope`.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ResourceScopeID(pub(crate) usize);



/// A reference counted token associated with a `ResourceScope`.
/// Allows [`System`] to track when a `ResourceScope` is no longer needed.
#[derive(Clone, Debug)]
pub struct ResourceScopeToken {
	ref_count: Rc<()>,
	id: ResourceScopeID,
}

impl ResourceScopeToken {
	pub(crate) fn new(id: ResourceScopeID) -> ResourceScopeToken {
		ResourceScopeToken {
			ref_count: Rc::new(()),
			id
		}
	}

	pub fn id(&self) -> ResourceScopeID {
		self.id
	}
}

impl From<&'_ ResourceScopeToken> for Option<ResourceScopeID> {
	fn from(token: &ResourceScopeToken) -> Option<ResourceScopeID> {
		Some(token.id)
	}
}



/// Keeps track of all resources which should be destroyed when all associated [`ResourceScopeTokens`][ResourceScopeToken]
/// are dropped.
///
/// ## Note
/// It is up to the owner to actually trigger cleanup - dropping a [`ResourceScope`] does nothing.
pub(crate) struct ResourceScope {
	resources: Vec<ScopedResourceHandle>,
	token: ResourceScopeToken,
}

impl ResourceScope {
	pub fn new(token: ResourceScopeToken) -> ResourceScope {
		ResourceScope {
			resources: Vec::new(),
			token
		}
	}

	pub fn ref_count(&self) -> usize {
		Rc::strong_count(&self.token.ref_count)
	}

	pub fn insert(&mut self, handle: ScopedResourceHandle) {
		self.resources.push(handle);
	}

	pub fn destroy_owned_resources(&mut self, resources: &mut Resources) {
		// println!("\nDestroying resources for {:?}", self.token.id);

		for resource in self.resources.drain(..) {
			use ScopedResourceHandle::*;

			// println!("==== Deleting {resource:?}");

			match resource {
				Buffer{handle} => unsafe {
					raw::DeleteBuffers(1, &handle);
				}

				Vao{handle} => unsafe {
					raw::DeleteVertexArrays(1, &handle);
				}

				Query{handle} => unsafe {
					raw::DeleteQueries(1, &handle);
				}

				Texture{key} => {
					let texture = resources.textures.remove(key)
						.expect("Trying to destroy texture that has already been removed");

					unsafe {
						raw::DeleteSamplers(1, &texture.sampler_handle);
						raw::DeleteTextures(1, &texture.texture_handle);
					}
				}

				Framebuffer{key} => {
					let framebuffer = resources.framebuffers.remove(key)
						.expect("Trying to destroy framebuffer that has already been removed");

					unsafe {
						raw::DeleteFramebuffers(1, &framebuffer.handle);
					}
				}
			}
		}

		self.resources.clear();
	}
}

impl std::ops::Drop for ResourceScope {
	fn drop(&mut self) {
		assert!(self.resources.is_empty(), "ResourceScope has been dropped without being cleaned up!");
	}
}


#[derive(Debug)]
pub(crate) enum ScopedResourceHandle {
	Buffer{handle: u32},
	Vao{handle: u32},
	Query{handle: u32},

	Texture{key: TextureKey},
	Framebuffer{key: FramebufferKey},
}
