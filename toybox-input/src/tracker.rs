use winit::event::{VirtualKeyCode, MouseButton};



#[derive(Default)]
pub struct Tracker {
	pub active_buttons: Vec<Button>,
	pub down_buttons: Vec<Button>,
	pub up_buttons: Vec<Button>,
}

impl Tracker {
	pub fn reset(&mut self) {
		self.down_buttons.clear();
		self.up_buttons.clear();
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