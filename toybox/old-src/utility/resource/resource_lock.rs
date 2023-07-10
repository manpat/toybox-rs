use crate::utility::BorrowState;


// TODO(pat.m): remove nullability of resource_borrow_state
// I don't think it's really needed any more


pub struct ResourceLock<T> {
	pub(super) resource: *const T, // Nonnull
	pub(super) store_borrow_state: *const BorrowState, // Nonnull
	pub(super) resource_borrow_state: *const BorrowState, // Can be null
}


impl<T> std::ops::Deref for ResourceLock<T> {
	type Target = T;

	fn deref(&self) -> &'_ T {
		unsafe { &*self.resource }
	}
}


impl<T> Drop for ResourceLock<T> {
	fn drop(&mut self) {
		// POINTER SAFETY: `store_borrow_state` is guaranteed to be valid here since `ResourcesRefCell` must be pinned
		// in order to construct a ResourceLock, and if it is dropped before this point it will panic.
		// `resource_borrow_state` is also safe to dereference since its lifetime is <= `store_borrow_state`, but the
		// owning storage cannot be modified while `store_borrow_state` is borrowed.
		
		// Unborrow resource if it is individually locked
		if let Some(resource_borrow_state) = unsafe{self.resource_borrow_state.as_ref()} {
			resource_borrow_state.unborrow();
		}

		unsafe {
			(*self.store_borrow_state).unborrow();
		}
	}
}



pub struct ResourceLockMut<T> {
	pub(super) resource: *mut T, // Nonnull
	pub(super) store_borrow_state: *const BorrowState, // Nonnull
	pub(super) resource_borrow_state: *const BorrowState, // Can be null
}


impl<T> std::ops::Deref for ResourceLockMut<T> {
	type Target = T;

	fn deref(&self) -> &'_ T {
		unsafe { &*self.resource }
	}
}


impl<T> std::ops::DerefMut for ResourceLockMut<T> {
	fn deref_mut(&mut self) -> &'_ mut T {
		unsafe { &mut *self.resource }
	}
}


impl<T> Drop for ResourceLockMut<T> {
	fn drop(&mut self) {
		// POINTER SAFETY: `store_borrow_state` is guaranteed to be valid here since `ResourcesRefCell` must be pinned
		// in order to construct a ResourceLockMut, and if it is dropped before this point it will panic.
		// `resource_borrow_state` is also safe to dereference since its lifetime is <= `store_borrow_state`, but the
		// owning storage cannot be modified while `store_borrow_state` is borrowed.

		// Unborrow resource if it is individually locked
		if let Some(resource_borrow_state) = unsafe{self.resource_borrow_state.as_ref()} {
			resource_borrow_state.unborrow_mut();

			// NOTE: mutable borrows of resources only immutably borrow ResourceInner
			unsafe {
				(*self.store_borrow_state).unborrow();
			}
		} else {
			unsafe {
				(*self.store_borrow_state).unborrow_mut();
			}
		}
	}
}
