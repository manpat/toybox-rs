use crate::prelude::*;
use crate::input::{ActionID};
use std::collections::HashMap;

#[cfg(doc)]
use input::action::ActionKind::*;


// TODO(pat.m): Rename FrameState. its not very clear what this means

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActionState {
	Entered,
	Active,
	Left,
}


/// The complete state of input for a frame - after system inputs have been parsed into actions.
#[derive(Clone, Debug, Default)]
pub struct FrameState {
	/// All the button actions that are active or that changed this frame.
	pub button: HashMap<ActionID, ActionState>,

	/// [`Mouse`] or [`Pointer`] state if it is currently available, and the action its bound to.
	pub mouse: Option<(ActionID, Vec2)>,
}


impl FrameState {
	/// - For [`Trigger`]s: returns whether action was triggered this frame.
	/// - For [`State`]s: returns whether action is currently active (button is being held).
	/// - For [`Pointer`]/[`Mouse`] actions: returns whether action has precedence and has input.
	pub fn active(&self, action: ActionID) -> bool {
		let button_active = self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Entered | ActionState::Active));

		let mouse_active = matches!(self.mouse, Some((id, _)) if id == action);

		button_active || mouse_active
	}

	/// Whether a [`State`] or [`Trigger`] was activated this frame.
	/// # Note
	/// Cannot be used for [`Mouse`] or [`Pointer`] events.
	// TODO(pat.m): Maybe it should be though
	pub fn entered(&self, action: ActionID) -> bool {
		self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Entered))
	}

	/// Whether a [`State`] or [`Trigger`] was deactivated this frame.
	/// # Note
	/// Cannot be used for [`Mouse`] or [`Pointer`] events.
	// TODO(pat.m): Maybe it should be though
	pub fn left(&self, action: ActionID) -> bool {
		self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Left))
	}

	/// The current state for the given [`Mouse`] or [`Pointer`] action, if it is currently
	/// active and the passed action has precedence. Whether the return value is absolute
	/// or relative and which space the value is in is entirely dependent on the passed action.
	// TODO(pat.m): this needs to be made more sophisticated
	pub fn mouse(&self, action: ActionID) -> Option<Vec2> {
		self.mouse
			.filter(|&(mouse_action, _)| mouse_action == action)
			.map(|(_, state)| state)
	}
}


