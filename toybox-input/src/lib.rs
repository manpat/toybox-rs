use winit::window::{Window, CursorGrabMode};
use winit::event::*;
use winit::dpi::PhysicalPosition;
use common::math::Vec2;

use std::rc::Rc;

pub mod debug;
pub mod tracker;

pub mod prelude {}

pub use tracker::*;
pub use winit::event::{VirtualKeyCode as Key, MouseButton};

pub struct System {
	pub tracker: Tracker,

	window: Rc<Window>,
	wants_capture: bool,
}

/// Input tracker queries. Just convenience functions for the same calls on `self.tracker`
impl System {
	pub fn button_down(&self, button: impl Into<Button>) -> bool {
		self.tracker.button_down(button)
	}

	pub fn button_just_down(&self, button: impl Into<Button>) -> bool {
		self.tracker.button_just_down(button)
	}

	pub fn button_just_up(&self, button: impl Into<Button>) -> bool {
		self.tracker.button_just_up(button)
	}
}

impl System {
	pub fn set_capture_mouse(&mut self, capture: bool) {
		self.wants_capture = capture;

		if capture {
			self.window.set_cursor_grab(CursorGrabMode::Confined)
				.or_else(|_e| self.window.set_cursor_grab(CursorGrabMode::Locked))
				.expect("Failed to grab cursor");

			self.window.set_cursor_visible(false);

		} else {
			self.window.set_cursor_grab(CursorGrabMode::None)
				.expect("Failed to release cursor grab");

			self.window.set_cursor_visible(true);
		}
	}
}


/// Internal. Will be called by core.
impl System {
	pub fn new(window: Rc<Window>) -> System {
		System {
			tracker: Tracker::default(),
			window,
			wants_capture: false,
		}
	}

	// Clear any 'this frame' state in the tracker and prepare for recieving new inputs
	pub fn reset_tracker(&mut self) {
		self.tracker.reset();
	}

	/// Called when something (e.g., egui) changes its mind about whether or not it wants to claim input.
	/// We're assuming that when we become _not_ occluded we can safely manage things without interference.
	pub fn set_occluded(&mut self, occluded: bool) {
		if !occluded {
			self.set_capture_mouse(self.wants_capture);
		}
	}

	pub fn on_window_event(&mut self, event: &WindowEvent<'_>) {
		match event {
			WindowEvent::KeyboardInput{ input: KeyboardInput{ virtual_keycode: Some(key), state, .. }, .. } => {
				self.tracker.track_button(*key, *state == ElementState::Pressed);
			}

			WindowEvent::MouseInput{ button, state, .. } => {
				self.tracker.track_button(*button, *state == ElementState::Pressed);
			}

			WindowEvent::CursorMoved{ position, .. } => {
				let PhysicalPosition{x, y} = position.cast::<f32>();
				self.tracker.track_pointer_move(Vec2::new(x, y));
			}

			WindowEvent::CursorLeft{..} => self.tracker.track_pointer_left(),

			WindowEvent::Focused(false) => self.tracker.track_focus_lost(),
			WindowEvent::Focused(true) => self.tracker.track_focus_gained(),

			_ => {}
		}
	}

	pub fn on_device_event(&mut self, event: &DeviceEvent) {
		match event {
			DeviceEvent::MouseMotion{delta: (dx, dy)} => {
				self.tracker.track_mouse_move(Vec2::new(*dx as f32, *dy as f32));
			}

			_ => {}
		}
	}

	// Do any processing that needs to happen to the raw input. No new inputs will be recieved this frame.
	pub fn process(&mut self) {

	}

}