use crate::prelude::*;


pub const STANDARD_VS_SHADER_SOURCE: &str = include_str!("shaders/standard.vs.glsl");
pub const FULLSCREEN_VS_SHADER_SOURCE: &str = include_str!("shaders/fullscreen.vs.glsl");
pub const FLAT_TEXTURED_FS_SHADER_SOURCE: &str = include_str!("shaders/flat.fs.glsl");



#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct StandardVertex {
	pub pos: Vec3,
	pub uv_packed: [u16; 2],
	pub color_packed: [u16; 4],
	pub _padding: [u32; 2],
}

impl StandardVertex {
	pub fn new(pos: Vec3, uv: Vec2, color: impl Into<Color>) -> StandardVertex {
		let [u, v] = uv.into();
		let [r, g, b, a] = color.into().to_array();

		StandardVertex {
			pos,
			uv_packed: [
				unorm_to_u16(u),
				unorm_to_u16(v),
			],

			color_packed: [
				unorm_to_u16(r),
				unorm_to_u16(g),
				unorm_to_u16(b),
				unorm_to_u16(a),
			],

			_padding: [0; 2],
		}
	}

	pub fn from_pos(pos: Vec3) -> StandardVertex {
		Self::new(pos, Vec2::zero(), Color::white())
	}

	pub fn with_color(pos: Vec3, color: impl Into<Color>) -> StandardVertex {
		Self::new(pos, Vec2::zero(), color)
	}

	pub fn with_uv(pos: Vec3, uv: Vec2) -> StandardVertex {
		Self::new(pos, uv, Color::white())
	}
}

fn unorm_to_u16(o: f32) -> u16 {
	let umax_f = u16::MAX as f32;
	(o * umax_f).clamp(0.0, umax_f) as u16
}