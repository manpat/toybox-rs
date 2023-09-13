use crate::prelude::*;
use crate::Axis;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SamplerName {
	pub raw: u32,
}

impl super::ResourceName for SamplerName {
	const GL_IDENTIFIER: u32 = gl::SAMPLER;
	fn as_raw(&self) -> u32 { self.raw }
}


#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AddressingMode {
	Repeat = gl::REPEAT,
	Clamp = gl::CLAMP_TO_EDGE,
	Mirror = gl::MIRRORED_REPEAT,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FilterMode {
	Nearest,
	Linear,
}


/// Samplers
impl super::Core {
	pub fn create_sampler(&self) -> SamplerName {
		SamplerName {
			raw: unsafe {
				let mut name = 0;
				self.gl.CreateSamplers(1, &mut name);
				name
			}
		}
	}

	pub fn destroy_sampler(&self, name: SamplerName) {
		unsafe {
			self.gl.DeleteSamplers(1, &name.raw)
		}
	}

	pub fn bind_sampler(&self, unit: u32, name: SamplerName) {
		assert!(unit < self.capabilities.max_image_units as u32);

		// TODO(pat.m): state tracking
		unsafe {
			self.gl.BindSampler(unit, name.raw);
		}
	}

	pub fn set_sampler_addressing_mode(&self, name: SamplerName, mode: AddressingMode) {
		self.set_sampler_axis_addressing_mode(name, Axis::X, mode);
		self.set_sampler_axis_addressing_mode(name, Axis::Y, mode);
		self.set_sampler_axis_addressing_mode(name, Axis::Z, mode);
	}

	pub fn set_sampler_axis_addressing_mode(&self, name: SamplerName, axis: Axis, mode: AddressingMode) {
		let parameter = match axis {
			Axis::X => gl::TEXTURE_WRAP_S,
			Axis::Y => gl::TEXTURE_WRAP_T,
			Axis::Z => gl::TEXTURE_WRAP_R,
		};

		unsafe {
			self.gl.SamplerParameteri(name.raw, parameter, mode as i32);
		}
	}

	pub fn set_sampler_minify_filter(&self, name: SamplerName, filter: FilterMode, mip_filter: impl Into<Option<FilterMode>>) {
		use FilterMode::*;

		let value = match (filter, mip_filter.into()) {
			(Nearest, None) => gl::NEAREST,
			(Linear, None) => gl::LINEAR,
			(Nearest, Some(Nearest)) => gl::NEAREST_MIPMAP_NEAREST,
			(Linear, Some(Nearest)) => gl::LINEAR_MIPMAP_NEAREST,
			(Nearest, Some(Linear)) => gl::NEAREST_MIPMAP_LINEAR,
			(Linear, Some(Linear)) => gl::LINEAR_MIPMAP_LINEAR,
		};

		unsafe {
			self.gl.SamplerParameteri(name.raw, gl::TEXTURE_MIN_FILTER, value as i32);
		}
	}

	pub fn set_sampler_magnify_filter(&self, name: SamplerName, filter: FilterMode) {
		use FilterMode::*;

		let value = match filter {
			Nearest => gl::NEAREST,
			Linear => gl::LINEAR,
		};

		unsafe {
			self.gl.SamplerParameteri(name.raw, gl::TEXTURE_MAG_FILTER, value as i32);
		}
	}
}