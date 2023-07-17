use crate::prelude::*;

pub struct Context {
	pub gfx: gfx::System,
}

impl Context {
	pub(crate) fn start_frame(&mut self) {}

	pub(crate) fn notify_resized(&mut self, new_size: Vec2i) {
		self.gfx.resize(new_size);
	}

	pub(crate) fn finalize_frame(&mut self) {
		self.gfx.execute_frame();
	}

	pub(crate) fn shutdown(&mut self) {}
}