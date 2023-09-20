use winit::event::*;

pub mod debug;
pub mod tracker;

pub mod prelude {
	
}

pub use tracker::*;
pub use winit::event::{VirtualKeyCode as Key, MouseButton};

pub struct System {
	pub tracker: Tracker,
}

impl System {
	pub fn new() -> System {
		System {
			tracker: Tracker::default(),
		}
	}
}


/// Internal. Will be called by core.
impl System {
	// Clear any 'this frame' state in the tracker and prepare for recieving new inputs
	pub fn reset_tracker(&mut self) {
		self.tracker.reset();
	}

	pub fn on_window_event(&mut self, event: &WindowEvent<'_>) {
		match event {
			WindowEvent::KeyboardInput{ input: KeyboardInput{ virtual_keycode: Some(key), state, .. }, .. } => {
				self.tracker.track_button(*key, *state == ElementState::Pressed);
			}

			WindowEvent::MouseInput{ button, state, .. } => {
				self.tracker.track_button(*button, *state == ElementState::Pressed);
			}

			WindowEvent::Focused(false) => self.tracker.track_focus_lost(),
			WindowEvent::Focused(true) => self.tracker.track_focus_gained(),

			_ => {}
		}
	}

	pub fn on_device_event(&mut self, _event: &DeviceEvent) {

	}

	// Do any processing that needs to happen to the raw input. No new inputs will be recieved this frame.
	pub fn process(&mut self) {

	}

}