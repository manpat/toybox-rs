use crate::prelude::*;

/// Global state
impl super::Core {
	pub fn set_viewport(&self, size: Vec2i) {
		unsafe {
			self.gl.Viewport(0, 0, size.x, size.y);
		}
	}

	pub fn set_blend_mode(&self, state: impl Into<Option<BlendMode>>) {
		let state = state.into();

		if self.current_blend_mode.get() == state {
			return
		}

		self.current_blend_mode.set(state);

		self.set_feature(gl::BLEND, state.is_some());

		if let Some(state) = state {
			let BlendMode{source_color, source_alpha, destination_color, destination_alpha, color_function, alpha_function} = state;

			unsafe {
				self.gl.BlendEquationSeparate(color_function as u32, alpha_function as u32);
				self.gl.BlendFuncSeparate(
					source_color as u32,
					destination_color as u32,

					source_alpha as u32,
					destination_alpha as u32,
				);
			}
		}
	}

	pub fn set_depth_test(&self, enabled: bool) {
		if self.depth_test_enabled.get() != enabled {
			self.set_feature(gl::DEPTH_TEST, enabled);
			self.depth_test_enabled.set(enabled);
		}
	}

	pub fn set_depth_write(&self, enabled: bool) {
		if self.depth_write_enabled.get() != enabled {
			unsafe {
				self.gl.DepthMask(if enabled { gl::TRUE } else { gl::FALSE });
			}

			self.depth_write_enabled.set(enabled);
		}
	}

	fn set_feature(&self, feature: u32, enable: bool) {
		unsafe {
			if enable {
				self.gl.Enable(feature);
			} else {
				self.gl.Disable(feature);
			}
		}
	}
}


#[repr(u32)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BlendFactor {
	Zero = gl::ZERO,
	One = gl::ONE,
	SourceAlpha = gl::SRC_ALPHA,
	OneMinusSourceAlpha = gl::ONE_MINUS_SRC_ALPHA,

	DestinationAlpha = gl::DST_ALPHA,
	OneMinusDestinationAlpha = gl::ONE_MINUS_DST_ALPHA,

	SourceColor = gl::SRC_COLOR,
	OneMinusSourceColor = gl::ONE_MINUS_SRC_COLOR,

	DestinationColor = gl::DST_COLOR,
	OneMinusDestinationColor = gl::ONE_MINUS_DST_COLOR,

	// TODO(pat.m): the rest?
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum BlendFunction {
	Add = gl::FUNC_ADD,
	// TODO(pat.m): the rest?
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct BlendMode {
	pub source_color: BlendFactor,
	pub source_alpha: BlendFactor,

	pub destination_color: BlendFactor,
	pub destination_alpha: BlendFactor,

	pub color_function: BlendFunction,
	pub alpha_function: BlendFunction,
}

impl BlendMode {
	pub const ALPHA: BlendMode = BlendMode {
		source_color: BlendFactor::SourceAlpha,
		destination_color: BlendFactor::OneMinusSourceAlpha,

		source_alpha: BlendFactor::One,
		destination_alpha: BlendFactor::OneMinusSourceAlpha,

		// TODO(pat.m): ?????? idk whats correct here
		// source_alpha: BlendFactor::OneMinusDestinationAlpha,
		// destination_alpha: BlendFactor::One,
		
		color_function: BlendFunction::Add,
		alpha_function: BlendFunction::Add,
	};

	pub const PREMULTIPLIED_ALPHA: BlendMode = BlendMode {
		source_color: BlendFactor::One,
		destination_color: BlendFactor::OneMinusSourceAlpha,

		source_alpha: BlendFactor::OneMinusDestinationAlpha,
		destination_alpha: BlendFactor::One,

		.. BlendMode::ALPHA
	};

	pub const ADDITIVE: BlendMode = BlendMode::combined(BlendFactor::SourceAlpha, BlendFactor::One);
	pub const MULTIPLY: BlendMode = BlendMode::combined(BlendFactor::DestinationColor, BlendFactor::Zero);

	pub const fn combined(source: BlendFactor, destination: BlendFactor) -> BlendMode {
		BlendMode {
			source_color: source,
			source_alpha: source,

			destination_color: destination,
			destination_alpha: destination,

			color_function: BlendFunction::Add,
			alpha_function: BlendFunction::Add,
		}
	}
}

