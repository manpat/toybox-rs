use crate::prelude::*;

pub struct Context {
	pub gfx: gfx::System,
	pub audio: audio::System,
	pub input: input::System,
	pub egui: egui::Context,

	pub(super) egui_integration: egui_backend::Integration,

	pub(super) egui_claiming_input_gate: Gate,

	// TODO(pat.m): might want to be able to disable this.
	/// Whether or not to show the built in debug menu.
	/// Can be toggled by F10.
	pub show_debug_menu: bool,
	pub wants_quit: bool,
}

impl Context {
	// Called at the very beginning of the frame, before any events are processed.
	pub(crate) fn prepare_frame(&mut self) {
		self.audio.update();
		self.input.reset_tracker();
	}

	// Called after events are processed, immediately before control is passed to the app.
	pub(crate) fn start_frame(&mut self) {
		self.input.process();
		self.egui = self.egui_integration.start_frame();

		if self.input.button_just_down(input::Key::F10) {
			self.show_debug_menu = !self.show_debug_menu;
		}

		if self.input.button_just_down(input::Key::Escape) {
			self.wants_quit = true;
		}
	}

	pub(crate) fn notify_resized(&mut self, new_size: Vec2i) {
		self.gfx.resize(new_size);
		self.input.on_resize(new_size);
	}

	// Called after app returns control, before the frame ends.
	pub(crate) fn finalize_frame(&mut self) {
		self.egui_integration.end_frame(&mut self.gfx);

		// We want to inform the input system if anything might be interferring with things like
		// cursor capture state.
		let claiming_input = self.egui.wants_keyboard_input() || self.egui.wants_pointer_input();
		match self.egui_claiming_input_gate.update(claiming_input) {
			GateState::RisingEdge => self.input.set_occluded(true),
			GateState::FallingEdge => self.input.set_occluded(false),
			_ => {}
		}

		self.gfx.execute_frame();
	}

	pub(crate) fn shutdown(&mut self) {}
}


