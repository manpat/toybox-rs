use crate::prelude::*;

/// Global state
impl super::Core {
	pub fn set_viewport(&self, size: Vec2i) {
		unsafe {
			self.gl.Viewport(0, 0, size.x, size.y);
		}
	}
}