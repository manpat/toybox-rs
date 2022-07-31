use common::math::*;
use crate::input::raw;
use crate::input::action::{ActionID, ActionKind};
use crate::input::context::{self, ContextID, InputContext};
use std::collections::HashMap;


pub struct InputSystem {
	/// All declared input contexts
	contexts: Vec<InputContext>,

	/// Active input contexts, ordered by priority in reverse order.
	/// Contexts at the end will recieve actions first
	active_contexts: Vec<ContextID>,

	/// Set when a context has been pushed or popped
	/// Will cause mouse capture state to be reevaluated when set
	active_contexts_changed: bool,

	/// Used for remapping mouse input
	window_size: Vec2,


	pub raw_state: raw::RawState,


	frame_state: FrameState,
	prev_frame_state: FrameState,


	sdl2_mouse: sdl2::mouse::MouseUtil,
}


impl InputSystem {
	pub fn new_context(&mut self, name: impl Into<String>) -> context::Builder<'_> {
		let context_id = context::ContextID(self.contexts.len());
		let context = InputContext::new_empty(name.into(), context_id);

		self.contexts.push(context);

		context::Builder::new(self.contexts.last_mut().unwrap())
	}

	pub fn enter_context(&mut self, context_id: ContextID) {
		assert!(!self.active_contexts.contains(&context_id));
		self.active_contexts.push(context_id);
		self.active_contexts_changed = true;
	}

	pub fn leave_context(&mut self, context_id: ContextID) {
		if let Some(context_pos) = self.active_contexts.iter().position(|&id| id == context_id) {
			self.active_contexts.remove(context_pos);
		}

		self.active_contexts_changed = true;
	}

	pub fn set_context_active(&mut self, context_id: ContextID, active: bool) {
		let currently_active = self.is_context_active(context_id);
		
		match (currently_active, active) {
			(false, true) => self.enter_context(context_id),
			(true, false) => self.leave_context(context_id),
			_ => {}
		}
	}

	pub fn is_context_active(&self, context_id: ContextID) -> bool {
		self.active_contexts.contains(&context_id)
	}

	pub fn frame_state(&self) -> &FrameState {
		&self.frame_state
	}

	pub fn contexts(&self) -> impl Iterator<Item = &'_ InputContext> {
		self.contexts.iter()
	}

	pub fn active_contexts(&self) -> impl Iterator<Item = &'_ InputContext> {
		self.active_contexts.iter()
			.filter_map(move |id| self.contexts.get(id.0))
	}

	pub fn is_mouse_captured(&self) -> bool {
		self.sdl2_mouse.relative_mouse_mode()
	}
}


impl InputSystem {
	pub(crate) fn new(sdl2_mouse: sdl2::mouse::MouseUtil) -> InputSystem {
		InputSystem {
			contexts: Vec::new(),
			active_contexts: Vec::new(),
			active_contexts_changed: false,

			// We're assuming on_resize will be called soon after construction by Engine
			window_size: Vec2::zero(),

			raw_state: raw::RawState::new(),

			frame_state: FrameState::default(),
			prev_frame_state: FrameState::default(),

			sdl2_mouse,
		}
	}

	pub(crate) fn clear(&mut self) {
		self.raw_state.track_new_frame();

		if self.active_contexts_changed {
			self.active_contexts_changed = false;

			let contexts = &self.contexts;
			self.active_contexts.sort_by_key(move |id| contexts.get(id.0).map(|ctx| ctx.priority()));

			// Find last active context using mouse input; we want to enable relative mouse mode if it's relative
			let should_capture_mouse = self.active_contexts.iter().rev()
				.flat_map(|&ContextID(id)| self.contexts.get(id))
				.find_map(InputContext::mouse_action)
				.map_or(false, |(action, _)| action.kind() == ActionKind::Mouse);

			self.sdl2_mouse.set_relative_mouse_mode(should_capture_mouse);
		}
	}

	pub(crate) fn on_resize(&mut self, window_size: Vec2i) {
		self.window_size = window_size.to_vec2();
	}

	pub(crate) fn handle_event(&mut self, event: &sdl2::event::Event) {
		use sdl2::event::{Event, WindowEvent};
		use sdl2::mouse::MouseWheelDirection;

		match *event {
			Event::Window{ win_event: WindowEvent::Leave, .. } => self.raw_state.track_mouse_leave(),
			Event::Window{ win_event: WindowEvent::FocusLost, .. } => self.raw_state.track_focus_lost(),

			Event::MouseWheel { y, direction: MouseWheelDirection::Normal, .. } => self.raw_state.track_wheel_move(y),
			Event::MouseWheel { y, direction: MouseWheelDirection::Flipped, .. } => self.raw_state.track_wheel_move(-y),

			Event::MouseMotion { xrel, yrel, x, y, .. } => {
				let absolute = Vec2i::new(x, y);
				let relative = Vec2i::new(xrel, yrel);
				self.raw_state.track_mouse_move(absolute, relative);
			}

			Event::MouseButtonDown { mouse_btn, .. } => self.raw_state.track_button_down(mouse_btn.into()),
			Event::MouseButtonUp { mouse_btn, .. } => self.raw_state.track_button_up(mouse_btn.into()),

			Event::KeyDown { scancode: Some(scancode), .. } => self.raw_state.track_button_down(scancode.into()),
			Event::KeyUp { scancode: Some(scancode), .. } => self.raw_state.track_button_up(scancode.into()),

			_ => {}
		}

		// &Event::MouseMotion { xrel, yrel, x, y, .. } => {
			// let Vec2{x: w, y: h} = self.window_size;
			// let aspect = w/h;

			// let mouse_x = x as f32 / w * 2.0 - 1.0;
			// let mouse_y = -(y as f32 / h * 2.0 - 1.0);

			// // Maintain a 1x1 safe region in center screen
			// let (mouse_x, mouse_y) = if aspect > 1.0 {
			// 	(mouse_x * aspect, mouse_y)
			// } else {
			// 	(mouse_x, mouse_y / aspect)
			// };

		// 	self.mouse_absolute = Some(Vec2::new(mouse_x, mouse_y));

		// 	let mouse_dx =  xrel as f32;
		// 	let mouse_dy = -yrel as f32;

		// 	let mouse_delta = Vec2::new(mouse_dx, mouse_dy);
		// 	let current_delta = self.mouse_delta.get_or_insert_with(Vec2::zero);
		// 	*current_delta += mouse_delta;
		// }
	}

	pub(crate) fn process_events(&mut self) {
		std::mem::swap(&mut self.frame_state, &mut self.prev_frame_state);

		self.frame_state.button.clear();
		self.frame_state.mouse = None;

		// Calculate mouse action
		let mouse_action = self.active_contexts.iter().rev()
			.flat_map(|&ContextID(id)| self.contexts.get(id))
			.find_map(|ctx| ctx.mouse_action().zip(Some(ctx)));

		if let Some(((action, action_id), context)) = mouse_action {
			let remap = |value: Vec2i, absolute| {
				// let Vec2{x: w, y: h} = self.window_size;
				// let aspect = w/h;

				let offset = match absolute {
					true => Vec2::new(-1.0, 1.0),
					false => Vec2::zero()
				};

				value.to_vec2() / self.window_size * Vec2::new(2.0, -2.0) + offset
			};

			if action.kind().is_relative() {
				let sensitivity = context.mouse_sensitivity().unwrap_or(1.0);
				self.frame_state.mouse = self.raw_state.mouse_delta.map(|state| (action_id, remap(state, false) * sensitivity));
			} else {
				self.frame_state.mouse = self.raw_state.mouse_absolute.map(|state| (action_id, remap(state, true)));
			}
		}

		// Collect new button actions
		for &button in self.raw_state.new_buttons.iter() {
			let most_appropriate_action = self.active_contexts.iter().rev()
				.flat_map(|&ContextID(id)| self.contexts.get(id))
				.find_map(|ctx| ctx.action_for_button(button));

			if let Some((_, action_id)) = most_appropriate_action {
				self.frame_state.button.insert(action_id, ActionState::Entered);
			}
		}

		// Collect stateful button actions - triggers _only_ run on button down events
		for &button in self.raw_state.active_buttons.iter() {
			let most_appropriate_action = self.active_contexts.iter().rev()
				.flat_map(|&ContextID(id)| self.contexts.get(id))
				.flat_map(|ctx| ctx.action_for_button(button))
				.find(|(action, _)| action.kind() == ActionKind::State);

			if let Some((_, action_id)) = most_appropriate_action {
				// If this button was previously entered or active, remain active
				if let Some(&state) = self.prev_frame_state.button.get(&action_id)
					&& state != ActionState::Left
				{
					self.frame_state.button.insert(action_id, ActionState::Active);
				} else {
					self.frame_state.button.insert(action_id, ActionState::Entered);
				}
			}
		}


		// Combine current active actions with previous frame state
		for (&action_id, _) in self.prev_frame_state.button.iter()
			.filter(|(_, state)| **state != ActionState::Left)
		{
			// If a previously active action doesn't appear in the new framestate
			// register it as a deactivation
			self.frame_state.button.entry(action_id)
				.or_insert(ActionState::Left);
		}
	}
}




#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ActionState {
	Entered,
	Active,
	Left,
}


/// The complete state of input for a frame - after system inputs have been parsed into actions
#[derive(Clone, Debug, Default)]
pub struct FrameState {
	/// All the button actions that are active or that changed this frame
	button: HashMap<ActionID, ActionState>,

	/// Mouse state if it is currently available, and the action its bound to
	mouse: Option<(ActionID, Vec2)>,
}


impl FrameState {
	/// For Triggers: returns whether action was triggered this frame
	/// For States: returns whether action is currently active (button is being held)
	pub fn active(&self, action: ActionID) -> bool {
		let button_active = self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Entered | ActionState::Active));

		let mouse_active = matches!(self.mouse, Some((id, _)) if id == action);

		button_active || mouse_active
	}

	/// Whether a state or trigger was actived this frame
	pub fn entered(&self, action: ActionID) -> bool {
		self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Entered))
	}

	/// Whether the state was deactivated this frame
	pub fn left(&self, action: ActionID) -> bool {
		self.button.get(&action)
			.map_or(false, |state| matches!(state, ActionState::Left))
	}

	pub fn mouse(&self, action: ActionID) -> Option<Vec2> {
		self.mouse
			.filter(|&(mouse_action, _)| mouse_action == action)
			.map(|(_, state)| state)
	}
}


