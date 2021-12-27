use crate::prelude::*;
use crate::gfx::*;

use slotmap::SlotMap;
use std::rc::Rc;
use std::cell::{Cell, UnsafeCell};

slotmap::new_key_type!{
	pub struct TextureKey;
	pub struct FramebufferKey;
}


pub type ResourceStore<T> = Rc<ResourcesRefCell<T>>;


#[derive(Debug)]
pub struct Resources {
	textures: Rc<ResourcesRefCell<Texture>>,
	framebuffers: Rc<ResourcesRefCell<Framebuffer>>,
}

impl Resources {
	pub(super) fn new() -> Resources {
		Resources {
			textures: ResourcesRefCell::new().into(),
			framebuffers: ResourcesRefCell::new().into(),
		}
	}

	pub(super) fn insert_texture(&mut self, texture: Texture) -> TextureKey {
		self.textures.insert(texture)
	}

	pub(super) fn insert_framebuffer(&mut self, framebuffer: Framebuffer) -> FramebufferKey {
		self.framebuffers.insert(framebuffer)
	}

	pub(super) fn on_resize(&mut self, backbuffer_size: Vec2i) {
		self.textures.mutate(|inner| {
			for (texture, _) in inner.storage.values_mut() {
				texture.on_resize(backbuffer_size);
			}
		});

		self.framebuffers.mutate(|inner| {
			for (framebuffer, _) in inner.storage.values_mut() {
				framebuffer.rebind_attachments(&self.textures);
			}
		});
	}

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




pub trait Resource {
	type Key : slotmap::Key;
}

impl Resource for Texture {
	type Key = TextureKey;
}

impl Resource for Framebuffer {
	type Key = FramebufferKey;
}




// See std::cell::RefCell
#[derive(Debug)]
pub struct ResourcesRefCell<T: Resource> {
	inner_borrow_state: BorrowState,
	inner: UnsafeCell<ResourcesInner<T>>,
}

#[derive(Debug)]
struct ResourcesInner<T: Resource> {
	storage: SlotMap<T::Key, (T, BorrowState)>
}


impl<T: Resource> ResourcesRefCell<T> {
	fn new() -> Self {
		ResourcesRefCell {
			inner_borrow_state: BorrowState::new(),
			inner: UnsafeCell::new(
				ResourcesInner {
					storage: SlotMap::with_key(),
				}
			)
		}
	}

	pub fn get(self: &Rc<Self>, key: T::Key) -> ResourceLock<T> {
		// Mark ResourceInner as being borrowed - no changes to the collections can be made from this point
		self.inner_borrow_state.borrow();

		// Now that we've locked inner for read, we can safely dereference it, and get a reference to the resource
		let inner = unsafe { &*self.inner.get() };
		let (resource, resource_borrow_state) = &inner.storage[key];

		// Mark resource itself as being borrowed
		resource_borrow_state.borrow();

		ResourceLock {
			inner: Rc::clone(self),
			resource,
			resource_borrow_state,
		}
	}

	pub fn get_mut(self: &Rc<Self>, key: T::Key) -> ResourceLockMut<T> {
		// Mark ResourceInner as being borrowed - no changes to the collections can be made from this point
		// NOTE: mutable borrows of resources only immutably borrow ResourceInner - since individual resources
		// track their own borrow state, and we only need to guarantee that those resources stay pinned while borrowed
		self.inner_borrow_state.borrow();

		// Now that we've locked inner for read, we can safely dereference it, and get a reference to the resource.
		// NOTE: we are making the assumption that `f` DOES NOT modify the storage of resources, and only creates a
		// new reference into the storage. anything else would probably cause UB.
		// We are also assuming that there is no reentrancy, since that could create aliasing
		// mutable references, which would also be UB.
		let inner = unsafe { &mut *self.inner.get() };
		let (resource, resource_borrow_state) = &mut inner.storage[key];

		// Mark resource itself as being borrowed
		resource_borrow_state.borrow_mut();

		ResourceLockMut {
			inner: Rc::clone(self),
			resource,
			resource_borrow_state,
		}
	}


	fn insert(&self, resource: T) -> T::Key {
		self.mutate(move |inner| inner.storage.insert((resource, BorrowState::new())))
	}


	fn mutate<F, R>(&self, f: F) -> R
		where F: FnOnce(&mut ResourcesInner<T>) -> R
	{
		// Mark ResourceInner as being mutably borrowed for the duration of function call
		self.inner_borrow_state.borrow_mut();

		// TODO(pat.m): FIGURE OUT IF IT IS ACTUALLY SOUND TO RETURN STUFF HERE
		// Its only for internal use so _should_ be fine, but even so
		let result = f(unsafe { &mut *self.inner.get() });

		self.inner_borrow_state.unborrow_mut();

		result
	}
}


pub struct ResourceLock<T: Resource> {
	inner: Rc<ResourcesRefCell<T>>,
	resource: *const T,
	resource_borrow_state: *const BorrowState, // Can be null
}

impl<T: Resource> std::ops::Deref for ResourceLock<T> {
	type Target = T;

	fn deref(&self) -> &'_ T {
		unsafe { &*self.resource }
	}
}

impl<T: Resource> Drop for ResourceLock<T> {
	fn drop(&mut self) {
		// Unborrow resource if it is individually locked
		if let Some(resource_borrow_state) = unsafe{self.resource_borrow_state.as_ref()} {
			resource_borrow_state.unborrow();
		}

		self.inner.inner_borrow_state.unborrow();
	}
}



pub struct ResourceLockMut<T: Resource> {
	inner: Rc<ResourcesRefCell<T>>,
	resource: *mut T,
	resource_borrow_state: *const BorrowState, // Can be null
}

impl<T: Resource> std::ops::Deref for ResourceLockMut<T> {
	type Target = T;

	fn deref(&self) -> &'_ T {
		unsafe { &*self.resource }
	}
}

impl<T: Resource> std::ops::DerefMut for ResourceLockMut<T> {
	fn deref_mut(&mut self) -> &'_ mut T {
		unsafe { &mut *self.resource }
	}
}

impl<T: Resource> Drop for ResourceLockMut<T> {
	fn drop(&mut self) {
		// Unborrow resource if it is individually locked
		if let Some(resource_borrow_state) = unsafe{self.resource_borrow_state.as_ref()} {
			resource_borrow_state.unborrow_mut();

			// NOTE: mutable borrows of resources only immutably borrow ResourceInner
			self.inner.inner_borrow_state.unborrow();
		} else {
			self.inner.inner_borrow_state.unborrow_mut();
		}
	}
}



#[derive(Debug)]
struct BorrowState(Cell<isize>);

impl BorrowState {
	fn new() -> Self {
		BorrowState(Cell::new(0))
	}

	fn borrow(&self) {
		let new_borrow_state = self.0.get() + 1;
		assert!(new_borrow_state > 0, "tried to immutably borrow while already mutably borrowed");
		self.0.set(new_borrow_state);
	}

	fn unborrow(&self) {
		let new_borrow_state = self.0.get() - 1;
		assert!(new_borrow_state >= 0);
		self.0.set(new_borrow_state);
	}

	fn borrow_mut(&self) {
		assert!(self.0.get() == 0, "tried to mutably borrow while already borrowed");
		self.0.set(-1);
	}

	fn unborrow_mut(&self) {
		assert!(self.0.get() == -1);
		self.0.set(0);
	}
}



