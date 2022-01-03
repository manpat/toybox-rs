use crate::audio::system::EvaluationContext;
use common::{math::*, Ease};

use std::marker::PhantomData;


pub trait ParameterData: Copy + std::fmt::Debug {}
impl ParameterData for f32 {}
impl ParameterData for Vec2 {}
impl ParameterData for Vec3 {}
impl ParameterData for Vec4 {}


pub enum Parameter<T: ParameterData> {
	Constant(T),
	Channel(ParameterChannelReciever<T>),
}

impl<T: ParameterData> Parameter<T> where f32: Ease<T> {
	pub fn get(&mut self /*, _eval_ctx: &'_ EvaluationContext<'_>*/) -> T {
		match self {
			Parameter::Constant(v) => *v,
			Parameter::Channel(ch) => ch.update(),
		}
	}
}

impl<T> From<T> for Parameter<T>
	where T: ParameterData
{
	fn from(o: T) -> Self {
		Parameter::Constant(o)
	}
}

impl<T> From<ParameterChannelReciever<T>> for Parameter<T>
	where T: ParameterData
{
	fn from(o: ParameterChannelReciever<T>) -> Self {
		Parameter::Channel(o)
	}
}




use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::cell::Cell;



const WRITE_LOCK: u32  = 0b001;
const READ_LOCK: u32   = 0b010;
const NOTIFY_FLAG: u32 = 0b100;


struct ParameterChannelInner<T: ParameterData> {
	new_target: Cell<T>,
	lock_state: AtomicU32,
}

impl<T: ParameterData> ParameterChannelInner<T> {
	fn write_and_notify(&self, value: T) {
		let initial_value = self.lock_state.fetch_or(WRITE_LOCK, Ordering::Acquire);
		assert!(initial_value & WRITE_LOCK == 0,
			"Attempting to acquire already acquired write lock. Only one thread may write to a ParameterChannel at once");

		if initial_value & !NOTIFY_FLAG == 0 {
			// Wasn't locked for read or write, so we have acquired the lock
			// NOTE: Ignoring notify flag because we always want to notify with the latest value
			// SAFETY: Write lock has been exclusively acquired at this point so nothing can be reading `new_target` at this point
			self.new_target.set(value);
			self.release_write_lock_and_notify();
		}

		// At this point the write lock is held, but read lock is still also held, so dumb spin until read lock is released.
		// This should be fine because read lock should always only be held for long enough to read `new_target`.
		loop {
			let current_value = self.lock_state.fetch_or(WRITE_LOCK, Ordering::Acquire);
			if current_value & !NOTIFY_FLAG == WRITE_LOCK {
				break
			}
		}

		// SAFETY: Write lock has been exclusively acquired at this point so nothing can be reading `new_target` at this point
		self.new_target.set(value);
		self.release_write_lock_and_notify();
	}

	fn release_write_lock_and_notify(&self) {
		self.lock_state.fetch_or(NOTIFY_FLAG, Ordering::Relaxed);
		self.lock_state.fetch_and(!WRITE_LOCK, Ordering::Release);
	}

	fn try_read(&self) -> Option<T> {
		// Try lock for read
		let initial_value = self.lock_state.fetch_or(READ_LOCK, Ordering::Acquire);
		assert!(initial_value&READ_LOCK == 0, "Attempting to acquire already acquired read lock");

		// If there's nothing to read, don't lock
		// If locked for write, don't lock
		if initial_value != NOTIFY_FLAG {
			// Release read lock
			self.lock_state.fetch_and(!READ_LOCK, Ordering::Release);
			return None;
		}

		// SAFETY: nothing can write to `new_target` while read lock is held at this point
		let new_target = self.new_target.get();

		// Release read lock _and_ clear notify flag
		self.lock_state.fetch_and(!READ_LOCK & !NOTIFY_FLAG, Ordering::Release);

		Some(new_target)
	}
}


unsafe impl<T: ParameterData> Sync for ParameterChannelInner<T> {}



pub struct ParameterChannelSender<T: ParameterData> {
	inner: Arc<ParameterChannelInner<T>>,
}

impl<T: ParameterData> ParameterChannelSender<T> {
	pub fn send(&self, new_target: T) {
		self.inner.write_and_notify(new_target);
	}
}


pub struct ParameterChannelReciever<T: ParameterData> {
	inner: Arc<ParameterChannelInner<T>>,

	old_value: T,
	target_value: T,
	position: f32, // [0, 1]
}

impl<T: ParameterData> ParameterChannelReciever<T> where f32: Ease<T> {
	fn update(&mut self) -> T {
		let current_value = if self.position < 1.0 {
			let current_value = self.position.ease_linear(self.old_value, self.target_value);
			self.position += 1.0 / 500.0;
			current_value
		} else {
			self.target_value
		};

		if let Some(new_target) = self.inner.try_read() {
			self.old_value = current_value;
			self.target_value = new_target;
			self.position = 0.0;
		}

		current_value
	}
}



pub fn parameter_channel<T: ParameterData>(initial_value: T) -> (ParameterChannelSender<T>, ParameterChannelReciever<T>) {
	let inner = ParameterChannelInner {
		new_target: Cell::new(initial_value),
		lock_state: AtomicU32::new(0),
	};

	let inner = Arc::new(inner);

	let sender = ParameterChannelSender { inner: Arc::clone(&inner) };
	let reciever = ParameterChannelReciever {
		inner,

		old_value: initial_value,
		target_value: initial_value,
		position: 1.0,
	};

	(sender, reciever)
}