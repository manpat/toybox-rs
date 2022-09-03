use common::math::*;
use crate::utility::IdCounter;
use crate::utility::resource_scope::*;
use crate::input::raw;
use crate::input::action::ActionKind;
use crate::input::context::{self, ContextID, InputContext};
use crate::input::context_group::{ContextGroup, ContextGroupID};
use crate::input::frame_state::{FrameState, ActionState};

#[cfg(doc)]
use crate::input::action::Action;

/// Facillitates translation of raw sdl2 input events into context specific, semantic [`Action`]s,
/// and handles changing the mouse capture state as appropriate.
/// [`Action`]s are generated in [`process_events`] and can be accessed
/// through the systems current [`FrameState`] (see [`InputSystem::frame_state`]).
///
/// [`process_events`]: crate::Engine::process_events
pub struct InputSystem {
	/// All declared input contexts.
	contexts: Vec<InputContext>,

	/// All declared context groups.
	context_groups: Vec<ContextGroup>,

	/// Active context groups.
	active_context_groups: Vec<ContextGroupID>,

	/// Active input contexts, ordered by priority in reverse order.
	/// Contexts at the end will recieve actions first.
	active_contexts: Vec<ContextID>,

	/// Same as `active_contexts`, but filtered by context group.
	/// Regenerated from `active_contexts` whenever `active_context_changed` is set.
	filtered_active_contexts: Vec<ContextID>,

	/// Set when a context has been pushed or popped, or the set of active context groups has changed.
	/// Will cause mouse capture state to be reevaluated when set.
	active_contexts_changed: bool,

	/// Used for remapping mouse input.
	window_size: Vec2i,

	/// Counter for new contexts.
	context_id_counter: IdCounter,

	/// Counter for new context groups.
	context_group_id_counter: IdCounter,


	pub raw_state: raw::RawState,


	frame_state: FrameState,
	prev_frame_state: FrameState,


	resource_scope_store: ResourceScopeStore<InputScopedResource>,


	sdl2_mouse: sdl2::mouse::MouseUtil,
}


impl InputSystem {
	pub fn new_context(&mut self, name: impl Into<String>, resource_scope_id: impl Into<Option<ResourceScopeID>>) -> context::Builder<'_> {
		let context_id = ContextID(self.context_id_counter.next());
		let context = InputContext::new_empty(name.into(), context_id);

		self.contexts.push(context);

		let resource_scope = self.resource_scope_store.get_mut(resource_scope_id);
		resource_scope.insert(InputScopedResource::Context(context_id));

		context::Builder::new(self.contexts.last_mut().unwrap())
	}

	pub fn delete_context(&mut self, context_id: ContextID) {
		self.set_context_active(context_id, false);

		if let Ok(position) = self.contexts.binary_search_by_key(&context_id, |ctx| ctx.id) {
			self.contexts.remove(position);
		}
	}

	pub fn new_context_group(&mut self, name: impl Into<String>) -> ContextGroupID {
		let context_group_id = ContextGroupID(self.context_group_id_counter.next());
		let context_group = ContextGroup::new_empty(name.into(), context_group_id);

		// let resource_scope = self.resource_scope_store.get_mut(resource_scope_id);
		// resource_scope.insert(InputScopedResource::ContextGroup(context_id));

		self.context_groups.push(context_group);
		context_group_id
	}

	pub fn is_context_active(&self, context_id: ContextID) -> bool {
		// If the context is part of a context group, check whether or not that is active first.
		if let Some(context) = self.context(context_id)
			&& let Some(context_group_id) = context.context_group_id
			&& !self.active_context_groups.contains(&context_group_id)
		{
			return false;
		}

		self.active_contexts.contains(&context_id)
	}

	pub fn is_context_group_active(&self, context_group_id: ContextGroupID) -> bool {
		self.active_context_groups.contains(&context_group_id)
	}

	pub fn enter_context(&mut self, context_id: ContextID) {
		self.set_context_active(context_id, true);
	}

	pub fn leave_context(&mut self, context_id: ContextID) {
		self.set_context_active(context_id, false);
	}

	pub fn set_context_active(&mut self, context_id: ContextID, active: bool) {
		// Check active_contexts directly instead of using is_context_active since this function
		// acts independently of context groups
		let context_position = self.active_contexts.iter().position(|&id| id == context_id);
		
		match (context_position, active) {
			(None, true) => self.active_contexts.push(context_id),
			(Some(pos), false) => {
				self.active_contexts.remove(pos);
			},
			_ => return
		}

		self.active_contexts_changed = true;
	}

	pub fn set_context_group_active(&mut self, context_group_id: ContextGroupID, active: bool) {
		let context_group_position = self.active_context_groups.iter().position(|&id| id == context_group_id);
		
		match (context_group_position, active) {
			(None, true) => self.active_context_groups.push(context_group_id),
			(Some(pos), false) => {
				self.active_context_groups.remove(pos);
			},
			_ => return
		}

		self.active_contexts_changed = true;
	}

	pub fn frame_state(&self) -> &FrameState {
		&self.frame_state
	}

	pub fn context(&self, context_id: ContextID) -> Option<&'_ InputContext> {
		self.contexts.binary_search_by_key(&context_id, |ctx| ctx.id)
			.ok()
			.map(|idx| &self.contexts[idx])
	}

	pub fn context_group(&self, context_group_id: ContextGroupID) -> Option<&'_ ContextGroup> {
		self.context_groups.binary_search_by_key(&context_group_id, |ctx| ctx.id)
			.ok()
			.map(|idx| &self.context_groups[idx])
	}

	pub fn contexts(&self) -> impl Iterator<Item = &'_ InputContext> {
		self.contexts.iter()
	}

	pub fn context_groups(&self) -> impl Iterator<Item = &'_ ContextGroup> {
		self.context_groups.iter()
	}

	pub fn active_contexts(&self) -> impl Iterator<Item = &'_ InputContext> {
		self.active_contexts.iter()
			.filter_map(move |&id| self.context(id))
	}

	pub fn is_mouse_captured(&self) -> bool {
		self.sdl2_mouse.relative_mouse_mode()
	}
}


impl InputSystem {
	pub(crate) fn new(sdl2_mouse: sdl2::mouse::MouseUtil, global_scope_token: ResourceScopeToken) -> InputSystem {
		InputSystem {
			contexts: Vec::new(),
			context_groups: Vec::new(),
			active_contexts: Vec::new(),
			active_context_groups: Vec::new(),
			filtered_active_contexts: Vec::new(),
			active_contexts_changed: false,

			// We're assuming on_resize will be called soon after construction by Engine
			window_size: Vec2i::zero(),

			context_id_counter: IdCounter::new(),
			context_group_id_counter: IdCounter::new(),

			raw_state: raw::RawState::new(),

			frame_state: FrameState::default(),
			prev_frame_state: FrameState::default(),

			resource_scope_store: ResourceScopeStore::new(global_scope_token),

			sdl2_mouse,
		}
	}

	pub(crate) fn clear(&mut self) {
		self.raw_state.track_new_frame();

		if self.active_contexts_changed {
			self.active_contexts_changed = false;

			let contexts = &self.contexts;
			self.active_contexts.sort_by_key(move |&id| {
				contexts.iter()
					.find(|ctx| ctx.id == id)
					.map(|ctx| ctx.priority())
			});

			// TODO(pat.m): this is a little wasteful - since its scanning active_contexts way more than it needs to
			// but I'm expecting numbers to be small. I can improve this later.
			// Maybe it would be better for is_context_active to be implemented in terms of this.
			self.filtered_active_contexts = self.active_contexts.iter()
				.copied()
				.filter(|&id| self.is_context_active(id))
				.collect();

			// Find last active context using mouse input; we want to enable relative mouse mode if it's relative
			let should_capture_mouse = self.filtered_active_contexts.iter().rev()
				.flat_map(|&id| self.context(id))
				.find_map(InputContext::mouse_action)
				.map_or(false, |(action, _)| action.kind == ActionKind::Mouse);

			self.sdl2_mouse.set_relative_mouse_mode(should_capture_mouse);
		}
	}

	pub(crate) fn on_resize(&mut self, window_size: Vec2i) {
		self.window_size = window_size;
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
	}

	pub(crate) fn process_events(&mut self) {
		std::mem::swap(&mut self.frame_state, &mut self.prev_frame_state);

		self.frame_state.button.clear();
		self.frame_state.mouse = None;

		// Calculate mouse action
		let mouse_action = self.filtered_active_contexts.iter().rev()
			.flat_map(|&id| self.context(id))
			.find_map(|ctx| ctx.mouse_action().zip(Some(ctx)));

		if let Some(((action, action_id), context)) = mouse_action {
			// Should never be able to fail.
			let mouse_space = action.binding_info.mouse_space()
				.expect("Mouse action encountered without appropriate MouseSpace");

			if action.kind.is_relative() {
				let sensitivity = context.mouse_sensitivity().unwrap_or(1.0);
				self.frame_state.mouse = self.raw_state.mouse_delta.map(|state| (action_id, mouse_space.resolve_relative(state, self.window_size) * sensitivity));
			} else {
				self.frame_state.mouse = self.raw_state.mouse_absolute.map(|state| (action_id, mouse_space.resolve_absolute(state, self.window_size)));
			}
		}

		// Collect new button actions
		for &button in self.raw_state.new_buttons.iter() {
			let most_appropriate_action = self.filtered_active_contexts.iter().rev()
				.flat_map(|&id| self.context(id))
				.find_map(|ctx| ctx.action_for_button(button));

			if let Some((_, action_id)) = most_appropriate_action {
				self.frame_state.button.insert(action_id, ActionState::Entered);
			}
		}

		// Collect stateful button actions - triggers _only_ run on button down events
		for &button in self.raw_state.active_buttons.iter() {
			let most_appropriate_action = self.filtered_active_contexts.iter().rev()
				.flat_map(|&id| self.context(id))
				.flat_map(|ctx| ctx.action_for_button(button))
				.find(|(action, _)| action.kind == ActionKind::State);

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

	pub(crate) fn register_resource_scope(&mut self, token: ResourceScopeToken) {
		self.resource_scope_store.register_scope(token)
	}

	pub(crate) fn cleanup_resource_scope(&mut self, scope_id: ResourceScopeID) {
		let context = InputScopedResourceContext {
			contexts: &mut self.contexts,
			active_contexts: &mut self.active_contexts,
			active_contexts_changed: &mut self.active_contexts_changed,
		};

		self.resource_scope_store.cleanup_scope(scope_id, context)
	}
}

impl std::ops::Drop for InputSystem {
	fn drop(&mut self) {
		let context = InputScopedResourceContext {
			contexts: &mut self.contexts,
			active_contexts: &mut self.active_contexts,
			active_contexts_changed: &mut self.active_contexts_changed,
		};
		

		self.resource_scope_store.cleanup_all(context);
	}
}



#[derive(Debug)]
enum InputScopedResource {
	Context(ContextID),
	ContextGroup(ContextGroupID),
}

struct InputScopedResourceContext<'c> {
	contexts: &'c mut Vec<InputContext>,
	active_contexts: &'c mut Vec<ContextID>,
	active_contexts_changed: &'c mut bool,

	// TODO(pat.m): context_groups
}

impl ScopedResource for InputScopedResource {
	type Context<'c> = InputScopedResourceContext<'c>;

	fn destroy(self, context: &mut InputScopedResourceContext<'_>) {
		match self {
			InputScopedResource::Context(context_id) => {
				// TODO(pat.m): duplicates set_context_active and delete_context - should find a way to deduplicate this logic
				if let Some(position) = context.active_contexts.iter().position(|&id| id == context_id) {
					context.active_contexts.remove(position);
					*context.active_contexts_changed = true;
				}

				if let Ok(position) = context.contexts.binary_search_by_key(&context_id, |ctx| ctx.id) {
					context.contexts.remove(position);
				}
			}

			InputScopedResource::ContextGroup(context_group_id) => {
				// if let Ok(position) = context.contexts.binary_search_by_key(&context_id, |ctx| ctx.id) {
				// 	context.contexts.remove(position);
				// }
				todo!()
			}
		}
	}
}