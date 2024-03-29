use winit::event::{VirtualKeyCode, MouseButton};
use common::math::*;


#[derive(Default)]
pub struct Tracker {
	pub active_buttons: Vec<Button>,
	pub down_buttons: Vec<Button>,
	pub up_buttons: Vec<Button>,

	// This is in physical pixels! in Y-down screen space
	pub physical_mouse_position: Option<Vec2>,

	// This is in raw 'dots' per frame - y-down. related to dpi
	pub mouse_delta: Option<Vec2>,
}

/// Input query API.
impl Tracker {
	pub fn button_down(&self, button: impl Into<Button>) -> bool {
		self.active_buttons.contains(&button.into())
	}

	pub fn button_just_down(&self, button: impl Into<Button>) -> bool {
		self.down_buttons.contains(&button.into())
	}

	pub fn button_just_up(&self, button: impl Into<Button>) -> bool {
		self.up_buttons.contains(&button.into())
	}
}

/// Input gathering API - called by core.
impl Tracker {
	pub fn reset(&mut self) {
		self.down_buttons.clear();
		self.up_buttons.clear();

		self.mouse_delta = None;
	}

	pub fn track_button(&mut self, button: impl Into<Button>, down: bool) {
		let button = button.into();

		if down {
			if !self.active_buttons.contains(&button) {
				self.down_buttons.push(button);
				self.active_buttons.push(button);
			}
		} else {
			self.up_buttons.push(button);
			self.active_buttons.retain(|active_button| *active_button != button);
		}
	}

	pub fn track_mouse_position(&mut self, pos: Vec2) {
		self.physical_mouse_position = Some(pos);
	}

	pub fn track_mouse_move(&mut self, mut delta: Vec2) {
		*self.mouse_delta.get_or_insert_with(Vec2::zero) += delta;
	}

	pub fn track_mouse_left(&mut self) {
		self.physical_mouse_position = None;
		self.mouse_delta = None;
	}

	pub fn track_focus_lost(&mut self) {
		self.active_buttons.clear();
	}

	pub fn track_focus_gained(&mut self) {
		
	}
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Button {
	Key(VirtualKeyCode),
	Mouse(MouseButton),
}

impl From<VirtualKeyCode> for Button {
	fn from(o: VirtualKeyCode) -> Button {
		Button::Key(o)
	} 
}

impl From<MouseButton> for Button {
	fn from(o: MouseButton) -> Button {
		Button::Mouse(o)
	} 
}