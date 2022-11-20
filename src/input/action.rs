use crate::input::{raw, context};
use common::math::*;

#[cfg(doc)]
use crate::input::frame_state::ActionState::*;


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionID {
	pub(super) context_id: context::ContextID,
	pub(super) index: usize,
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ActionKind {
	/// # One-off, immediate action.
	/// On button down, will emit an [`Entered`] state, immediately followed by a [`Left`] state.
	/// Triggers will never enter the [`Active`] state, but FrameState::active can still be used.
	/// Will not trigger if its owning context is activated while its bound keys are held - only triggers on button down while context is active
	Trigger,

	/// # Continuous binary input.
	/// Will emit [`Entered`] and [`Left`] states on button down/up, and will remain in the [`Active`]
	/// state while the button is held. If the actions bound button is held when its owning context is
	/// activated, it will emit [`Entered`] and [`Active`] states.
	/// Similarly, if the owning context is disabled while the action is active, it will emit a [`Left`] state.
	State,

	/// # Per-frame relative mouse input.
	/// The cursor will be put into 'relative' mode while the owning context is the topmost context with
	/// a mouse action. Input will only be available while the mouse is moving within the window.
	/// Cannot exist in a context with any other Mouse or Pointer actions.
	Mouse,

	/// # Absolute mouse position relative to window.
	/// Input will only be available while the mouse is within the window, and will be normalised to
	/// the logical height of the window. Cannot exist in a context with any other Mouse or Pointer actions.
	Pointer,
}

impl ActionKind {
	pub fn is_mouse_kind(&self) -> bool {
		matches!(*self, ActionKind::Mouse | ActionKind::Pointer)
	}

	pub fn is_button_kind(&self) -> bool {
		matches!(*self, ActionKind::Trigger | ActionKind::State)
	}

	pub fn is_relative(&self) -> bool {
		matches!(*self, ActionKind::Mouse)
	}
}


/// Determines how mouse input is transformed before being consumed by [`Mouse`](ActionKind::Mouse) and
/// [`Pointer`](ActionKind::Pointer) [`Action`]s.
#[derive(Debug, Copy, Clone)]
pub enum MouseSpace {
	/// Unaltered logical coordinates from raw input device.
	/// Origin is in top left, y expands down, and space extends up to but excluding window size.
	Window,

	/// Relates to NDC space, as in each axis is mapped into [-1, 1], and y expands up the screen.
	Normalized,

	/// Like `Normalized` except scales one axis to preserve the aspect ratio of the space. The smallest
	/// axis is always unit length, effectively creating a 1x1 safe region that is always accessible.
	PreserveAspect,

	/// Like `Window` but flipped along y and multiplied by a constant factor.
	/// Exists only to maintain compatibility with old code.
	/// Its direct use is not recommended.
	// TODO(pat.m): this is dodgy, but maybe we should keep something like this for resolution independence.
	// #[deprecated = "Exists only for backwards compatibility."]
	LegacyPixelRatio,
}

impl MouseSpace {
	pub fn resolve_relative(&self, window_space_delta: Vec2i, window_size: Vec2i) -> Vec2 {
		use MouseSpace::*;

		match self {
			Window => window_space_delta.to_vec2(),

			Normalized => window_space_delta.to_vec2() / window_size.to_vec2() * Vec2::new(2.0, -2.0),

			PreserveAspect => {
				let aspect = window_size.x as f32 / window_size.y as f32;

				// Maintain a 1x1 safe region in center screen
				let aspect_scalar = if aspect > 1.0 {
					Vec2::new(aspect, 1.0)
				} else {
					Vec2::new(1.0, 1.0 / aspect)
				};

				let normalized = Normalized.resolve_relative(window_space_delta, window_size);
				normalized * aspect_scalar
			}

			#[allow(deprecated)]
			LegacyPixelRatio => window_space_delta.to_vec2() * Vec2::new(1.0 / 100.0, -1.0 / 100.0),
		}
	}

	pub fn resolve_absolute(&self, window_space_pos: Vec2i, window_size: Vec2i) -> Vec2 {
		use MouseSpace::*;

		match self {
			Window => window_space_pos.to_vec2(),

			Normalized => self.resolve_relative(window_space_pos - window_size/2, window_size),

			PreserveAspect => self.resolve_relative(window_space_pos - window_size/2, window_size),

			#[allow(deprecated)]
			LegacyPixelRatio => unimplemented!("LegacyPixelRatio only provided for Mouse Action kinds"),
		}
	}
}


// TODO(pat.m): rename - this is more about kind specific info than bindings.
#[derive(Debug, Copy, Clone)]
pub enum BindingInfo {
	Button {
		default_binding: raw::Button,
	},

	Mouse {
		space: MouseSpace,
		default_sensitivity: f32,
	},

	Pointer {
		space: MouseSpace,
	},
}

impl BindingInfo {
	pub fn mouse_space(&self) -> Option<MouseSpace> {
		match self {
			BindingInfo::Mouse { space, .. }
			| BindingInfo::Pointer { space } => Some(*space),

			_ => None,
		}
	}
}


#[derive(Debug)]
pub struct Action {
	pub name: String,
	pub kind: ActionKind,

	pub binding_info: BindingInfo,
}


impl Action {
	pub fn new_trigger(name: impl Into<String>, default_binding: impl Into<raw::Button>) -> Action {
		Action {
			name: name.into(),
			kind: ActionKind::Trigger,
			binding_info: BindingInfo::Button { default_binding: default_binding.into() },
		}
	}

	pub fn new_state(name: impl Into<String>, default_binding: impl Into<raw::Button>) -> Action {
		Action {
			name: name.into(),
			kind: ActionKind::State,
			binding_info: BindingInfo::Button { default_binding: default_binding.into() },
		}
	}

	pub fn new_mouse(name: impl Into<String>, space: MouseSpace, default_sensitivity: f32) -> Action {
		Action {
			name: name.into(),
			kind: ActionKind::Mouse,
			binding_info: BindingInfo::Mouse { space, default_sensitivity },
		}
	}

	pub fn new_pointer(name: impl Into<String>, space: MouseSpace) -> Action {
		Action {
			name: name.into(),
			kind: ActionKind::Pointer,
			binding_info: BindingInfo::Pointer { space },
		}
	}
}






#[cfg(test)]
mod test {
	use common::assert_vec_eq;
	use crate::prelude::*;
	use super::*;

	#[test]
	fn test_mouse_space() {
		let window_size = Vec2i::new(200, 100);
		let landscape_aspect = window_size.x as f32 / window_size.y as f32;

		let portrait_window_size = window_size.transpose();
		let portrait_aspect = portrait_window_size.x as f32 / portrait_window_size.y as f32;

		let pos_center = Vec2i::new(100, 50);
		let pos_top_right = Vec2i::new(200, 0);
		let pos_bottom_left = Vec2i::new(0, 100);
		let pos_top_right_portrait = Vec2i::new(100, 0);

		let delta_zero = Vec2i::new(0, 0);
		let delta_up_right = Vec2i::new(100, -50);
		let delta_down_left = Vec2i::new(-100, 50);
		let delta_up_right_portrait = Vec2i::new(50, -100);

		// Window

		let window_center_absolute = MouseSpace::Window.resolve_absolute(pos_center, window_size);
		let window_center_relative = MouseSpace::Window.resolve_relative(delta_up_right, window_size);

		assert_vec_eq!(window_center_absolute, pos_center.to_vec2());
		assert_vec_eq!(window_center_relative, delta_up_right.to_vec2());

		// Normalized

		let normalized_center_absolute = MouseSpace::Normalized.resolve_absolute(pos_center, window_size);
		let normalized_center_relative = MouseSpace::Normalized.resolve_relative(delta_zero, window_size);

		assert_vec_eq!(normalized_center_absolute, Vec2::zero());
		assert_vec_eq!(normalized_center_relative, Vec2::zero());


		let normalized_tr_absolute = MouseSpace::Normalized.resolve_absolute(pos_top_right, window_size);
		let normalized_tr_relative = MouseSpace::Normalized.resolve_relative(delta_up_right, window_size);

		assert_vec_eq!(normalized_tr_absolute, Vec2::new(1.0, 1.0));
		assert_vec_eq!(normalized_tr_relative, Vec2::new(1.0, 1.0));


		let normalized_bl_absolute = MouseSpace::Normalized.resolve_absolute(pos_bottom_left, window_size);
		let normalized_bl_relative = MouseSpace::Normalized.resolve_relative(delta_down_left, window_size);

		assert_vec_eq!(normalized_bl_absolute, Vec2::new(-1.0, -1.0));
		assert_vec_eq!(normalized_bl_relative, Vec2::new(-1.0, -1.0));

		// PreserveAspect

		let preserve_aspect_center_absolute = MouseSpace::PreserveAspect.resolve_absolute(pos_center, window_size);
		let preserve_aspect_center_relative = MouseSpace::PreserveAspect.resolve_relative(delta_zero, window_size);

		assert_vec_eq!(preserve_aspect_center_absolute, Vec2::zero());
		assert_vec_eq!(preserve_aspect_center_relative, Vec2::zero());


		let preserve_aspect_tr_absolute = MouseSpace::PreserveAspect.resolve_absolute(pos_top_right, window_size);
		let preserve_aspect_tr_relative = MouseSpace::PreserveAspect.resolve_relative(delta_up_right, window_size);

		assert_vec_eq!(preserve_aspect_tr_absolute, Vec2::new(landscape_aspect, 1.0));
		assert_vec_eq!(preserve_aspect_tr_relative, Vec2::new(landscape_aspect, 1.0));


		let preserve_aspect_bl_absolute = MouseSpace::PreserveAspect.resolve_absolute(pos_bottom_left, window_size);
		let preserve_aspect_bl_relative = MouseSpace::PreserveAspect.resolve_relative(delta_down_left, window_size);

		assert_vec_eq!(preserve_aspect_bl_absolute, Vec2::new(-landscape_aspect, -1.0));
		assert_vec_eq!(preserve_aspect_bl_relative, Vec2::new(-landscape_aspect, -1.0));


		let preserve_aspect_tr_absolute_portrait = MouseSpace::PreserveAspect.resolve_absolute(pos_top_right_portrait, portrait_window_size);
		let preserve_aspect_tr_relative_portrait = MouseSpace::PreserveAspect.resolve_relative(delta_up_right_portrait, portrait_window_size);

		assert_vec_eq!(preserve_aspect_tr_absolute_portrait, Vec2::new(1.0, 1.0/portrait_aspect));
		assert_vec_eq!(preserve_aspect_tr_relative_portrait, Vec2::new(1.0, 1.0/portrait_aspect));
	}
}