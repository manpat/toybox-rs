use crate::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ImageFormat {
	Rgba(ComponentFormat),
	RedGreen(ComponentFormat),
	Red(ComponentFormat),

	R11G11B10F,
	Rgb10A2,
	Rgb10A2Ui,
	Srgb8,
	Srgba8,

	Depth,
	DepthStencil,
	Stencil,

	Depth16,
	Depth32,
}


impl ImageFormat {
	pub fn color() -> Self { ImageFormat::Rgba(ComponentFormat::Unorm8) }
	pub fn hdr_color() -> Self { ImageFormat::Rgba(ComponentFormat::F16) }
	pub fn srgb() -> Self { ImageFormat::Srgb8 }
	pub fn srgba() -> Self { ImageFormat::Srgba8 }

	pub fn unorm8() -> Self { ImageFormat::Red(ComponentFormat::Unorm8) }

	pub fn to_raw(&self) -> u32 {
		match self {
			ImageFormat::Rgba(ComponentFormat::Unorm8) => gl::RGBA8,
			ImageFormat::Rgba(ComponentFormat::Unorm16) => gl::RGBA16,

			ImageFormat::Rgba(ComponentFormat::I8) => gl::RGBA8I,
			ImageFormat::Rgba(ComponentFormat::I16) => gl::RGBA16I,
			ImageFormat::Rgba(ComponentFormat::I32) => gl::RGBA32I,

			ImageFormat::Rgba(ComponentFormat::U8) => gl::RGBA8UI,
			ImageFormat::Rgba(ComponentFormat::U16) => gl::RGBA16UI,
			ImageFormat::Rgba(ComponentFormat::U32) => gl::RGBA32UI,

			ImageFormat::Rgba(ComponentFormat::F16) => gl::RGBA16F,
			ImageFormat::Rgba(ComponentFormat::F32) => gl::RGBA32F,

			ImageFormat::RedGreen(ComponentFormat::Unorm8) => gl::RG8,
			ImageFormat::RedGreen(ComponentFormat::Unorm16) => gl::RG16,

			ImageFormat::RedGreen(ComponentFormat::I8) => gl::RG8I,
			ImageFormat::RedGreen(ComponentFormat::I16) => gl::RG16I,
			ImageFormat::RedGreen(ComponentFormat::I32) => gl::RG32I,

			ImageFormat::RedGreen(ComponentFormat::U8) => gl::RG8UI,
			ImageFormat::RedGreen(ComponentFormat::U16) => gl::RG16UI,
			ImageFormat::RedGreen(ComponentFormat::U32) => gl::RG32UI,

			ImageFormat::RedGreen(ComponentFormat::F16) => gl::RG16F,
			ImageFormat::RedGreen(ComponentFormat::F32) => gl::RG32F,

			ImageFormat::Red(ComponentFormat::Unorm8) => gl::R8,
			ImageFormat::Red(ComponentFormat::Unorm16) => gl::R16,

			ImageFormat::Red(ComponentFormat::I8) => gl::R8I,
			ImageFormat::Red(ComponentFormat::I16) => gl::R16I,
			ImageFormat::Red(ComponentFormat::I32) => gl::R32I,

			ImageFormat::Red(ComponentFormat::U8) => gl::R8UI,
			ImageFormat::Red(ComponentFormat::U16) => gl::R16UI,
			ImageFormat::Red(ComponentFormat::U32) => gl::R32UI,

			ImageFormat::Red(ComponentFormat::F16) => gl::R16F,
			ImageFormat::Red(ComponentFormat::F32) => gl::R32F,

			ImageFormat::R11G11B10F => gl::R11F_G11F_B10F,
			ImageFormat::Rgb10A2 => gl::RGB10_A2,
			ImageFormat::Rgb10A2Ui => gl::RGB10_A2UI,
			ImageFormat::Srgb8 => gl::SRGB8,
			ImageFormat::Srgba8 => gl::SRGB8_ALPHA8,

			ImageFormat::Depth => gl::DEPTH_COMPONENT24,
			ImageFormat::Stencil => gl::STENCIL_INDEX8,
			ImageFormat::DepthStencil => gl::DEPTH24_STENCIL8,

			ImageFormat::Depth16 => gl::DEPTH_COMPONENT16,
			ImageFormat::Depth32 => gl::DEPTH_COMPONENT32F,
		}
	}

	pub fn to_raw_component(&self) -> u32 {
		use ImageFormat::*;

		match self {
			Red(component) | RedGreen(component) | Rgba(component) => component.to_raw(),
			Srgb8 | Srgba8 | Stencil => gl::UNSIGNED_BYTE,
			_ => panic!("Unsupported"),
		}
	}

	pub fn to_raw_unsized(&self) -> u32 {
		match self {
			ImageFormat::Rgba(comp) if comp.is_normalized() => gl::RGBA,
			ImageFormat::RedGreen(comp) if comp.is_normalized() => gl::RG,
			ImageFormat::Red(comp) if comp.is_normalized() => gl::RED,

			ImageFormat::Rgba(_) => gl::RGBA_INTEGER,
			ImageFormat::RedGreen(_) => gl::RG_INTEGER,
			ImageFormat::Red(_) => gl::RED_INTEGER,

			ImageFormat::Rgb10A2 => gl::RGBA,
			ImageFormat::Rgb10A2Ui => gl::RGBA_INTEGER,
			ImageFormat::R11G11B10F => gl::RGB,
			ImageFormat::Srgb8 => gl::RGB,
			ImageFormat::Srgba8 => gl::RGBA,

			ImageFormat::Depth | ImageFormat::Depth16 | ImageFormat::Depth32 => gl::DEPTH_COMPONENT,
			ImageFormat::Stencil => gl::STENCIL_INDEX,
			ImageFormat::DepthStencil => gl::DEPTH_STENCIL,
		}
	}

	pub fn texel_byte_size(&self) -> usize {
		use ImageFormat::*;

		match self {
			Red(component) => component.byte_size(),
			RedGreen(component) => component.byte_size() * 2,
			Rgba(component) => component.byte_size() * 4,
			Srgb8 => 3,
			Rgb10A2 | Rgb10A2Ui | R11G11B10F | Srgba8 => 4,

			Stencil => 1,
			Depth16 => 2,
			Depth => 3,
			Depth32 | DepthStencil => 4,
		}
	}

	pub fn is_depth(&self) -> bool {
		use ImageFormat::*;
		matches!(self, Depth | Depth16 | Depth32)
	}

	pub fn is_depth_stencil(&self) -> bool {
		use ImageFormat::*;
		matches!(self, DepthStencil)
	}

	pub fn is_stencil(&self) -> bool {
		use ImageFormat::*;
		matches!(self, Stencil)
	}
}




#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ComponentFormat {
	Unorm8, Unorm16,
	I8, I16, I32,
	U8, U16, U32,
	F16, F32,
}

impl ComponentFormat {
	pub fn is_normalized(&self) -> bool {
		use ComponentFormat::*;

		match self {
			Unorm8 | Unorm16 | F16 | F32 => true,
			I8 | I16 | I32 | U8 | U16 | U32 => false,
		}
	}

	pub fn to_raw(&self) -> u32 {
		use ComponentFormat::*;

		match self {
			Unorm8 | U8 => gl::UNSIGNED_BYTE,
			Unorm16 | U16 => gl::UNSIGNED_SHORT,
			U32 => gl::UNSIGNED_INT,
			I8 => gl::BYTE,
			I16 => gl::SHORT,
			I32 => gl::INT,
			F32 => gl::FLOAT,

			F16 => panic!("Unsupported"),
		}
	}

	pub fn byte_size(&self) -> usize {
		use ComponentFormat::*;

		match self {
			Unorm8 | U8 | I8 => 1,
			Unorm16 | U16 | I16 | F16 => 2,
			U32 | I32 | F32 => 4,
		}
	}
}