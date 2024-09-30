use winit::window::{Window, CursorGrabMode};
use winit::event::*;
use winit::dpi::PhysicalPosition;
use common::*;

use std::rc::Rc;

pub mod debug;
pub mod tracker;
pub mod keys;

pub mod prelude {}

pub use tracker::*;
pub use winit::event::{MouseButton};
pub use winit::keyboard::{Key as LogicalKey, NamedKey as LogicalNamedKey, KeyCode as PhysicalKey};

/// Maps mouse dots to a raw angle in radians. Based on constants used by quake and hl source.
///
/// https://github.com/ValveSoftware/source-sdk-2013/blob/master/sp/src/game/client/in_mouse.cpp#L88
/// https://github.com/id-Software/Quake-III-Arena/blob/master/code/client/cl_main.c#L2331
pub const ANGLE_PER_MOUSE_DOT: f32 = 0.022 * PI / 180.0;

pub struct System {
	pub tracker: Tracker,
	// pub gil: gilrs::Gilrs,

	pub mouse_sensitivity: f32,

	window: Rc<Window>,
	wants_capture: bool,

	window_size: Vec2i,
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

	pub fn mouse_position_pixels(&self) -> Option<Vec2> {
		self.tracker.physical_mouse_position.map(|Vec2{x, y}| Vec2 {
			x,
			y: self.window_size.y as f32 - y - 1.0
		})
	}

	pub fn mouse_position_ndc(&self) -> Option<Vec2> {
		self.mouse_position_pixels().map(|px| {
			let flipped_ndc = px / self.window_size.to_vec2() - Vec2::splat(0.5);
			flipped_ndc * 2.0
		})
	}

	/// Gives raw mouse delta - transformed such that moving the mouse forward gives a positive y delta, and moving
	/// the mouse right gives a positive x delta.
	/// Returns None if window doesn't have mouse focus or if no mouse events occured last frame.
	pub fn mouse_delta_dots(&self) -> Option<Vec2> {
		self.tracker.mouse_delta.map(|dpf| dpf * Vec2::new(1.0, -1.0))
	}

	/// If available, gives mouse delta as an angle in radians based on ANGLE_PER_MOUSE_DOT and mouse_sensitivity.
	/// Returns None if window doesn't have mouse focus or if no mouse events occured last frame.
	///
	/// https://github.com/id-Software/Quake-III-Arena/blob/master/code/client/cl_input.c#L420
	pub fn mouse_delta_radians(&self) -> Option<Vec2> {
		self.mouse_delta_dots().map(|dpf| dpf * self.mouse_sensitivity * ANGLE_PER_MOUSE_DOT)
	}
}

impl System {
	pub fn set_capture_mouse(&mut self, capture: bool) {
		self.wants_capture = capture;

		if capture {
			if let Err(error) = self.window.set_cursor_grab(CursorGrabMode::Confined)
				.inspect_err(|error| log::warn!("Failed to capture mouse with 'confined' mode - falling back to 'locked' mode. {error}"))
				.or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Locked))
			{
				log::error!("Failed to lock mouse: {error}");
				return;
			}

			self.window.set_cursor_visible(false);

		} else {
			if let Err(error) = self.window.set_cursor_grab(CursorGrabMode::None) {
				log::error!("Failed to release cursor grab: {error}");
			}

			self.window.set_cursor_visible(true);
		}
	}
}


/// Internal. Will be called by core.
impl System {
	pub fn new(window: Rc<Window>) -> System {
		System {
			tracker: Tracker::default(),
			// gil: gilrs::Gilrs::new().unwrap(),
			window,
			wants_capture: false,

			// Default half way between quake and source sdk defaults
			// https://github.com/ValveSoftware/source-sdk-2013/blob/master/sp/src/game/client/in_mouse.cpp#L85
			// https://github.com/id-Software/Quake-III-Arena/blob/master/code/client/cl_main.c#L2308
			mouse_sensitivity: 5.0,

			window_size: Vec2i::splat(1),
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

	pub fn on_resize(&mut self, new_size: Vec2i) {
		self.window_size = new_size;
	}

	pub fn on_window_event(&mut self, event: &WindowEvent) {
		use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

		match event {
			WindowEvent::KeyboardInput{ event: event @ KeyEvent{ physical_key, state, .. }, .. } => {
				// Track logical key
				self.tracker.track_button(event.key_without_modifiers(), *state == ElementState::Pressed);

				// Track physical key
				self.tracker.track_button(*physical_key, *state == ElementState::Pressed);
			}

			WindowEvent::MouseInput{ button, state, .. } => {
				self.tracker.track_button(button.clone(), *state == ElementState::Pressed);
			}

			WindowEvent::CursorMoved{ position, .. } => {
				let PhysicalPosition{x, y} = position.cast::<f32>();
				self.tracker.track_mouse_position(Vec2::new(x, y));
			}

			WindowEvent::CursorLeft{..} => self.tracker.track_mouse_left(),

			WindowEvent::Focused(false) => self.tracker.track_focus_lost(),
			WindowEvent::Focused(true) => self.tracker.track_focus_gained(),

			// TODO(pat.m): track dpi

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
