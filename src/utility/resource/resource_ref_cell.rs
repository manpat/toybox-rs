use crate::utility::{Resource, BorrowState, ResourceLock, ResourceLockMut};

use slotmap::SlotMap;

use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::pin::Pin;


// See std::cell::RefCell
#[derive(Debug)]
pub(in super) struct ResourcesRefCell<T: Resource> {
	store_borrow_state: BorrowState,
	inner: UnsafeCell<ResourcesInner<T>>,

	// ResourcesRefCell must be pinned because ResourceLock[Mut] relies on its address staying stable
	// for the duration of its lifetime. Lifetimes are enforced by asserting not borrows are active on drop.
	_pin: PhantomPinned,
}

#[derive(Debug)]
struct ResourcesInner<T: Resource> {
	storage: SlotMap<T::Key, (T, BorrowState)>
}


impl<T: Resource> ResourcesRefCell<T> {
	pub fn new() -> Pin<Box<Self>> {
		Box::pin(ResourcesRefCell {
			store_borrow_state: BorrowState::new(),
			inner: UnsafeCell::new(
				ResourcesInner {
					storage: SlotMap::with_key(),
				}
			),

			_pin: PhantomPinned,
		})
	}

	pub fn get(self: Pin<&Self>, key: T::Key) -> ResourceLock<T> {
		// Mark ResourceInner as being borrowed - no changes to the collections can be made from this point
		self.store_borrow_state.borrow();

		// Now that we've locked inner for read, we can safely dereference it, and get a reference to the resource
		let inner = unsafe { &*self.inner.get() };
		let (resource, resource_borrow_state) = &inner.storage[key];

		// Mark resource itself as being borrowed
		resource_borrow_state.borrow();

		ResourceLock {
			resource,
			resource_borrow_state,
			store_borrow_state: &self.store_borrow_state,
		}
	}

	pub fn get_mut(self: Pin<&Self>, key: T::Key) -> ResourceLockMut<T> {
		// Mark ResourceInner as being borrowed - no changes to the collections can be made from this point
		// NOTE: mutable borrows of resources only immutably borrow ResourceInner - since individual resources
		// track their own borrow state, and we only need to guarantee that those resources stay pinned while borrowed
		self.store_borrow_state.borrow();

		// Now that we've locked inner for read, we can safely dereference it, and get a reference to the resource.
		// TODO(pat.m): need to check that this is actually sound - not sure how much is actually guaranteeing there are
		// no other mutable references to inner at this point.
		let inner = unsafe { &mut *self.inner.get() };
		let (resource, resource_borrow_state) = &mut inner.storage[key];

		// Mark resource itself as being borrowed
		resource_borrow_state.borrow_mut();

		ResourceLockMut {
			resource,
			resource_borrow_state,
			store_borrow_state: &self.store_borrow_state,
		}
	}


	pub fn insert(&self, resource: T) -> T::Key {
		self.mutate(move |inner| inner.storage.insert((resource, BorrowState::new())))
	}


	pub fn foreach_mut<F>(&self, mut f: F)
		where F: FnMut(&mut T)
	{
		self.mutate(move |inner| {
			for (resource, _) in inner.storage.values_mut() {
				f(resource);
			}
		});
	}


	fn mutate<F, R>(&self, f: F) -> R
		where F: FnOnce(&mut ResourcesInner<T>) -> R
			, R: 'static
	{
		// Mark ResourceInner as being mutably borrowed for the duration of function call
		self.store_borrow_state.borrow_mut();

		// TODO(pat.m): FIGURE OUT IF IT IS ACTUALLY SOUND TO RETURN STUFF HERE
		// Its only for internal use so _should_ be fine, but even so
		let result = f(unsafe { &mut *self.inner.get() });

		self.store_borrow_state.unborrow_mut();

		result
	}
}



impl<T: Resource> Drop for ResourcesRefCell<T> {
	fn drop(&mut self) {
		assert!(!self.store_borrow_state.is_borrowed(), "ResourceStore dropped while borrows still active");
	}
}