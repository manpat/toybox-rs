pub use sdl2::mouse::MouseButton;
pub use sdl2::keyboard::Scancode;
pub use sdl2::keyboard::Keycode;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Button {
	Mouse(MouseButton),
	Key(Scancode),
}

impl Button {
	pub fn is_mouse(&self) -> bool {
		matches!(self, Button::Mouse(_))
	}

	pub fn is_key(&self) -> bool {
		matches!(self, Button::Key(_))
	}
}


impl From<MouseButton> for Button {
	fn from(mb: MouseButton) -> Button {
		Button::Mouse(mb)
	}
}

impl From<Scancode> for Button {
	fn from(sc: Scancode) -> Button {
		Button::Key(sc)
	}
}

impl From<Keycode> for Button {
	fn from(virtual_key: Keycode) -> Button {
		Button::Key(Scancode::from_keycode(virtual_key).expect("Failed to map virtual keycode to scancode"))
	}
}


impl<T> From<&T> for Button
	where Button: From<T>, T: Copy
{
	fn from(b: &T) -> Button {
		Button::from(*b)
	}
}




use common::math::Vec2i;


#[derive(Debug)]
pub struct RawState {
	/// The current mouse position in screenspace
	/// Normalised to window height, and will be None if a capturing input context is active
	/// and also if focus is lost
	pub mouse_absolute: Option<Vec2i>,

	/// The mouse delta recorded this frame - if there is one
	/// Used for mouse capturing input contexts
	pub mouse_delta: Option<Vec2i>,

	/// Buttons currently being held
	pub active_buttons: Vec<Button>,

	/// Buttons that have become pressed this frame
	pub new_buttons: Vec<Button>,
}


impl RawState {
	pub fn new() -> RawState {
		RawState {
			mouse_absolute: None,
			mouse_delta: None,

			active_buttons: Vec::new(),
			new_buttons: Vec::new(),
		}
	}

	pub fn track_mouse_leave(&mut self) {
		self.mouse_absolute = None;
	}

	pub fn track_focus_lost(&mut self) {
		self.active_buttons.clear();
	}

	pub fn track_mouse_move(&mut self, absolute: Vec2i, relative: Vec2i) {
		self.mouse_absolute = Some(absolute);

		let current_delta = self.mouse_delta.get_or_insert_with(Vec2i::zero);
		*current_delta += relative;
	}

	pub fn track_button_down(&mut self, button: Button) {
		let button_is_active = self.active_buttons.contains(&button);

		if !button_is_active {
			self.active_buttons.push(button);
			self.new_buttons.push(button);
		}
	}

	pub fn track_button_up(&mut self, button: Button) {
		let button_is_active = self.active_buttons.contains(&button);

		if button_is_active {
			self.active_buttons.retain(|&b| b != button);
		}
	}

	pub fn track_new_frame(&mut self) {
		self.mouse_delta = None;
		self.new_buttons.clear();
	}
}