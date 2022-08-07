use common::math::*;
use crate::gfx;


/// Marks and describes a type that can safetly used as the vertex type in a [`gfx::Mesh`] or [`gfx::BasicMesh`].
/// 
/// ## Note
/// Types that implement this trait must also be marked `#[repr(C)]` as these types will be sent across the ABI boundary.
pub trait Vertex: Copy {
	fn descriptor() -> Descriptor;
}

#[derive(Copy, Clone, Debug)]
pub struct Descriptor {
	pub attributes: &'static [Attribute],
	pub size_bytes: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Attribute {
	pub offset_bytes: u32,
	pub num_elements: u32,
	pub gl_type: u32,
	pub normalized: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum AttributeType {
	Float,
	Vec2,
	Vec3,
	Vec4,
	Unorm8(u32),
}

impl AttributeType {
	const fn into_gl(self) -> (u32, u32) {
		use AttributeType::*;

		let gl_type = match self {
			Float => gfx::raw::FLOAT,
			Vec2 => gfx::raw::FLOAT,
			Vec3 => gfx::raw::FLOAT,
			Vec4 => gfx::raw::FLOAT,
			Unorm8(_) => gfx::raw::UNSIGNED_BYTE,
		};

		let num_elements = match self {
			Float => 1,
			Vec2 => 2,
			Vec3 => 3,
			Vec4 => 4,
			Unorm8(components) => components,
		};

		(gl_type, num_elements)
	}

	const fn is_normalized(self) -> bool {
		matches!(self, AttributeType::Unorm8(_))
	}
}


impl Attribute {
	pub const fn new(offset_bytes: u32, attribute_type: AttributeType) -> Attribute {
		let (gl_type, num_elements) = attribute_type.into_gl();
		let normalized = attribute_type.is_normalized();
		Attribute { offset_bytes, num_elements, gl_type, normalized }
	}
}




/// A simple color 3D vertex type.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ColorVertex {
	pub pos: Vec3,
	pub color: Color,
}

impl ColorVertex {
	pub fn new(pos: Vec3, color: impl Into<Color>) -> ColorVertex {
		let color = color.into();
		ColorVertex { pos, color }
	}
}

static COLOR_VERTEX_ATTRIBUTES: &'static [Attribute] = &[
	Attribute::new(0, AttributeType::Vec3),
	Attribute::new(12, AttributeType::Vec4),
];

impl Vertex for ColorVertex {
	fn descriptor() -> Descriptor {
		Descriptor {
			attributes: COLOR_VERTEX_ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}



/// A simple color 2D vertex type.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ColorVertex2D {
	pub pos: Vec2,
	pub color: Color,
}

impl ColorVertex2D {
	pub fn new(pos: Vec2, color: impl Into<Color>) -> ColorVertex2D {
		let color = color.into();
		ColorVertex2D { pos, color }
	}
}


static COLOR_VERTEX_2D_ATTRIBUTES: &'static [Attribute] = &[
	Attribute::new(0, AttributeType::Vec2),
	Attribute::new(8, AttributeType::Vec4),
];

impl Vertex for ColorVertex2D {
	fn descriptor() -> Descriptor {
		Descriptor {
			attributes: COLOR_VERTEX_2D_ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}