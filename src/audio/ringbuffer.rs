
use std::sync::atomic::{AtomicU32, Ordering};


const READ_LOCK: u32 = 0b01;
const WRITE_LOCK: u32 = 0b10;

const LOOP_FLAG: u32 = 1 << (u32::BITS-1);


/// A threadsafe lockfree ringbuffer implementation
pub struct Ringbuffer<T: Copy> {
	/// Owning pointer to data, bounded by `size`.
	data: *mut T,
	size: u32,

	/// Encodes start of ready data, and the end of the unused writable data.
	/// Most significant bit stores a 1b 'loop flag' to track parity of loop count.
	/// it is toggled every time this pointer crosses `size`.
	read_ptr: AtomicU32,

	/// Encodes end of ready data, and the start of the unused writable data.
	/// Most significant bit stores a 1b 'loop flag' to track parity of loop count.
	/// it is toggled every time this pointer crosses `size`.
	write_ptr: AtomicU32,

	/// Bitset determining whether buffer is locked for read or write
	lock: AtomicU32,
}

impl<T: Copy + Default> Ringbuffer<T> {
	pub fn new(size: usize) -> Self {
		// maximum size of usize::MAX/2 is to avoid potential overflow when calculating splits when locking
		assert!(size < (u32::MAX/2) as usize, "Requested ringbuffer too big");

		let data = vec![T::default(); size].into_boxed_slice();
		let data = unsafe {
			(*Box::into_raw(data)).as_mut_ptr()
		};

		Ringbuffer {
			data,
			size: size as u32,

			read_ptr: AtomicU32::new(0),
			write_ptr: AtomicU32::new(0),
			lock: AtomicU32::new(0),
		}
	}

	pub fn free_capacity(&self) -> usize {
		let head_encoded = self.write_ptr.load(Ordering::Relaxed);
		let tail_encoded = self.read_ptr.load(Ordering::Relaxed);

		let (head, head_loop) = Self::decode_ptr(head_encoded);
		let (tail, tail_loop) = Self::decode_ptr(tail_encoded);

		if head < tail {
			(tail - head) as usize
		} else if head == tail {
			if head_loop == tail_loop {
				self.size as usize
			} else {
				0
			}
		} else {
			(self.size - head + tail) as usize
		}
	}

	pub fn available_samples(&self) -> usize {
		self.size as usize - self.free_capacity()
	}

	pub fn capacity(&self) -> usize {
		self.size as usize
	}

	unsafe fn get_ref(&self, begin: u32, end: u32) -> &[T] {
		assert!(begin <= self.size);
		assert!(end <= self.size);
		assert!(begin <= end);

		let begin_ptr = self.data.offset(begin as isize);
		let size = (end - begin) as usize;

		std::slice::from_raw_parts(begin_ptr, size)
	}

	unsafe fn get_mut(&self, begin: u32, end: u32) -> &mut [T] {
		assert!(begin <= self.size);
		assert!(end <= self.size);
		assert!(begin <= end);

		let begin_ptr = self.data.offset(begin as isize);
		let size = (end - begin) as usize;

		std::slice::from_raw_parts_mut(begin_ptr, size)
	}

	fn decode_ptr(composed: u32) -> (u32, bool) {
		let value = composed & !LOOP_FLAG;
		let loop_flag = (composed & LOOP_FLAG) != 0;
		(value, loop_flag)
	}

	fn encode_ptr(position: u32, loop_flag: bool) -> u32 {
		assert!(position & LOOP_FLAG == 0);
		position | if loop_flag { LOOP_FLAG } else { 0 }
	}

	fn lock(&self, lock_for_write: bool, max_samples: usize) -> InternalAcquiredLock {
		// TODO(pat.m): justify why only reading `self.size` is safe here
		let max_samples = max_samples.min(self.size as usize) as u32;

		let lock_flag = match lock_for_write {
			false => READ_LOCK,
			true => WRITE_LOCK,
		};

		let prev_lock = self.lock.fetch_or(lock_flag, Ordering::Acquire);
		assert!(prev_lock & lock_flag == 0, "Trying to acquire lock while already locked");

		let mut atomic_head = &self.read_ptr;
		let mut atomic_tail = &self.write_ptr;

		if lock_for_write {
			std::mem::swap(&mut atomic_head, &mut atomic_tail);
		}

		let head_encoded = atomic_head.load(Ordering::Relaxed);
		let tail_encoded = atomic_tail.load(Ordering::Relaxed);

		let (head, head_loop) = Self::decode_ptr(head_encoded);
		let (tail, tail_loop) = Self::decode_ptr(tail_encoded);

		assert!(head < self.size);
		assert!(tail < self.size);

		// If we're locking for read and the ringbuffer is empty, or we're locking for write and
		// the ringbuffer is full, return empty range
		let range_contiguous = match lock_for_write {
			false => { head_loop == tail_loop }
			true => { head_loop != tail_loop }
		};

		let range_available = (head == tail) && range_contiguous;

		if range_available {
			return InternalAcquiredLock {
				old_head: head,
				new_head_encoded: head_encoded,
				presplit_tail: head,
				postsplit_tail: 0,
			};
		}

		let projected_new_tail = head + max_samples;

		let (presplit_tail, postsplit_tail) = if head < tail {
			// No wrap possible, so just clamp to tail
			(projected_new_tail.min(tail), 0)
		} else {
			// Chance to wrap.
			// If `projected_new_tail` < self.size, then `saturating_sub` will return zero
			// and `postsplit` will be empty meaning no wrap occured.
			// Otherwise it will be the modulo of `self.size` and `presplit` will be `self.size`
			let overflow = projected_new_tail.saturating_sub(self.size);
			(projected_new_tail.min(self.size), overflow.min(tail))
		};

		assert!(postsplit_tail <= head);

		let wrap_occured = presplit_tail == self.size;

		// If `presplit_tail` is `self.size`, then either wrapping occured and `postsplit_tail` represents
		// the end of the locked range, or the locked range perfectly fits into the end of storage in which case
		// we want to wrap it round back to 0, which `postspit_tail` happens to be. Otherwise no wrapping
		// occured and `presplit_tail` represents the end of the locked range.
		let new_head = if wrap_occured { postsplit_tail } else { presplit_tail };
		let new_head_encoded = Self::encode_ptr(new_head, head_loop ^ wrap_occured);

		InternalAcquiredLock {
			old_head: head,
			new_head_encoded,
			presplit_tail,
			postsplit_tail,
		}
	}

	pub fn lock_for_read(&self, max_samples: usize) -> RingbufferReadLock<'_, T> {
		let InternalAcquiredLock {old_head, new_head_encoded, presplit_tail, postsplit_tail} = self.lock(false, max_samples);

		let presplit = unsafe { self.get_ref(old_head, presplit_tail) };
		let postsplit = unsafe { self.get_ref(0, postsplit_tail) };

		RingbufferReadLock {
			presplit,
			postsplit,

			new_head_encoded,

			read_ptr: &self.read_ptr,
			lock: &self.lock,
		}
	}

	pub fn lock_for_write(&self, max_samples: usize) -> RingbufferWriteLock<'_, T> {
		let InternalAcquiredLock {old_head, new_head_encoded, presplit_tail, postsplit_tail} = self.lock(true, max_samples);

		let presplit = unsafe { self.get_mut(old_head, presplit_tail) };
		let postsplit = unsafe { self.get_mut(0, postsplit_tail) };

		RingbufferWriteLock {
			presplit,
			postsplit,

			new_head_encoded,

			write_ptr: &self.write_ptr,
			lock: &self.lock,
		}
	}
}

impl<T: Copy> Drop for Ringbuffer<T> {
	fn drop(&mut self) {
		unsafe {
			let slice_ptr = std::ptr::slice_from_raw_parts_mut(self.data, self.size as usize);
			let _ = Box::from_raw(slice_ptr);
		}
	}
}

unsafe impl<T: Copy> Send for Ringbuffer<T> {}
unsafe impl<T: Copy> Sync for Ringbuffer<T> {}


struct InternalAcquiredLock {
	old_head: u32,
	new_head_encoded: u32,
	presplit_tail: u32,
	postsplit_tail: u32,
}





#[derive(Debug)]
pub struct RingbufferReadLock<'rb, T> {
	pub presplit: &'rb [T],
	pub postsplit: &'rb [T],

	new_head_encoded: u32,

	read_ptr: &'rb AtomicU32,
	lock: &'rb AtomicU32,
}

impl<T> Drop for RingbufferReadLock<'_, T> {
	fn drop(&mut self) {
		self.read_ptr.store(self.new_head_encoded, Ordering::Relaxed);

		let prev_lock = self.lock.fetch_and(!READ_LOCK, Ordering::Release);
		assert!(prev_lock & READ_LOCK > 0, "Releasing read lock somehow not already held");
	}
}

impl<T: Copy> RingbufferReadLock<'_, T> {
	pub fn len(&self) -> usize {
		self.presplit.len() + self.postsplit.len()
	}
}




#[derive(Debug)]
pub struct RingbufferWriteLock<'rb, T> {
	pub presplit: &'rb mut [T],
	pub postsplit: &'rb mut [T],

	new_head_encoded: u32,

	write_ptr: &'rb AtomicU32,
	lock: &'rb AtomicU32,
}

impl<T> Drop for RingbufferWriteLock<'_, T> {
	fn drop(&mut self) {
		self.write_ptr.store(self.new_head_encoded, Ordering::Relaxed);

		let prev_lock = self.lock.fetch_and(!WRITE_LOCK, Ordering::Release);
		assert!(prev_lock & WRITE_LOCK > 0, "Releasing write lock somehow not already held");
	}
}

impl<T: Copy> RingbufferWriteLock<'_, T> {
	pub fn len(&self) -> usize {
		self.presplit.len() + self.postsplit.len()
	}
}





#[cfg(test)]
mod test {
	use super::*;

	fn new_buffer() -> Ringbuffer<i32> {
		let ringbuffer = Ringbuffer::new(5);
		unsafe {
			let slice = ringbuffer.get_mut(0, ringbuffer.size);
			for (i, v) in slice.iter_mut().enumerate() {
				*v = i as i32;
			}
		}
		ringbuffer
	}

	#[test]
	fn empty_read() {
		let ringbuffer = new_buffer();
		let lock = ringbuffer.lock_for_read(5);
		assert!(lock.is_empty());
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 5);
	}

	#[test]
	fn empty_write() {
		let ringbuffer = new_buffer();
		let lock = ringbuffer.lock_for_write(5);
		assert!(lock.len() == 5);
		assert_eq!(lock.presplit, &[0, 1, 2, 3, 4]);
		assert_eq!(lock.postsplit, &[]);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 0 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 0);
	}

	#[test]
	fn offset_empty_write() {
		let ringbuffer = new_buffer();
		ringbuffer.write_ptr.store(4, Ordering::Relaxed);
		ringbuffer.read_ptr.store(4, Ordering::Relaxed);

		let lock = ringbuffer.lock_for_write(5);
		assert!(lock.len() == 5);
		assert_eq!(lock.presplit, &[4]);
		assert_eq!(lock.postsplit, &[0, 1, 2, 3]);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 4 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 4);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 0);
	}

	#[test]
	fn full_read() {
		let ringbuffer = new_buffer();
		ringbuffer.write_ptr.store(0 | LOOP_FLAG, Ordering::Relaxed);
		ringbuffer.read_ptr.store(0, Ordering::Relaxed);

		let lock = ringbuffer.lock_for_read(5);
		assert_eq!(lock.presplit, &[0, 1, 2, 3, 4]);
		assert_eq!(lock.postsplit, &[]);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 0 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 0 | LOOP_FLAG);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 5);
	}

	#[test]
	fn offset_full_read() {
		let ringbuffer = new_buffer();
		ringbuffer.write_ptr.store(4 | LOOP_FLAG, Ordering::Relaxed);
		ringbuffer.read_ptr.store(4, Ordering::Relaxed);

		let lock = ringbuffer.lock_for_read(5);
		assert_eq!(lock.presplit, &[4]);
		assert_eq!(lock.postsplit, &[0, 1, 2, 3]);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 4 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 4 | LOOP_FLAG);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 5);
	}

	#[test]
	fn full_write() {
		let ringbuffer = new_buffer();
		ringbuffer.write_ptr.store(0 | LOOP_FLAG, Ordering::Relaxed);
		ringbuffer.read_ptr.store(0, Ordering::Relaxed);

		let lock = ringbuffer.lock_for_write(5);
		assert_eq!(lock.presplit, &[]);
		assert_eq!(lock.postsplit, &[]);
		assert_eq!(lock.new_head_encoded, 0 | LOOP_FLAG);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 0 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 0);
	}

	#[test]
	fn offset_full_write() {
		let ringbuffer = new_buffer();
		ringbuffer.write_ptr.store(4 | LOOP_FLAG, Ordering::Relaxed);
		ringbuffer.read_ptr.store(4, Ordering::Relaxed);

		let lock = ringbuffer.lock_for_write(5);
		assert_eq!(lock.presplit, &[]);
		assert_eq!(lock.postsplit, &[]);
		assert_eq!(lock.new_head_encoded, 4 | LOOP_FLAG);
		drop(lock);

		assert_eq!(ringbuffer.write_ptr.load(Ordering::Relaxed), 4 | LOOP_FLAG);
		assert_eq!(ringbuffer.read_ptr.load(Ordering::Relaxed), 4);
		assert_eq!(ringbuffer.lock.load(Ordering::Relaxed), 0);
		assert_eq!(ringbuffer.free_capacity(), 0);
	}
}

