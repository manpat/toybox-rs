pub mod resource_lock;
pub mod resource_ref_cell;

pub use resource_lock::*;
use resource_ref_cell::ResourcesRefCell;

use std::pin::Pin;



pub trait Resource {
	type Key : slotmap::Key + 'static;
}



#[derive(Debug)]
pub struct ResourceStore<T: Resource> {
	inner: Pin<Box<ResourcesRefCell<T>>>,
}


impl<T: Resource> ResourceStore<T> {
	pub fn new() -> Self {
		ResourceStore {
			inner: ResourcesRefCell::new()
		}
	}

	pub fn get(&self, key: T::Key) -> ResourceLock<T> {
		self.inner.as_ref().get(key)
	}

	pub fn get_mut(&mut self, key: T::Key) -> ResourceLockMut<T> {
		self.inner.as_ref().get_mut(key)
	}

	pub fn insert(&mut self, resource: T) -> T::Key {
		self.inner.insert(resource)
	}

	pub fn foreach_mut<F>(&mut self, f: F)
		where F: FnMut(&mut T)
	{
		self.inner.foreach_mut(f)
	}
}

