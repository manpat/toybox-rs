use crate::prelude::*;

pub struct Context {
	pub gfx_core: gfx::Core,
}

impl Context {
	pub(crate) fn start_frame(&mut self) {}

	pub(crate) fn notify_resized(&mut self, _new_size: Vec2i) {}

	pub(crate) fn finalize_frame(&mut self) {
		self.gfx_core.finalize_frame();
	}

	pub(crate) fn shutdown(&mut self) {}
}