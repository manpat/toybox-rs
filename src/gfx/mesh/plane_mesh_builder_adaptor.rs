use common::*;
use crate::gfx::mesh::{PolyBuilder2D, PolyBuilder3D, ColoredPolyBuilder};


/// Adapts some [`PolyBuilder3D`] to the [`PolyBuilder2D`] interface, given some 3D plane to build 2D geometry onto.
///
/// The orientation of the plane is defined by the `uv` columns of the matrix `uvw`, and its offset from origin
/// is defined by the `w` column.
pub struct PlaneMeshBuilderAdaptor<MB: PolyBuilder3D> {
	builder_3d: MB,
	uvw: Mat3,
}


impl<MB: PolyBuilder3D> PlaneMeshBuilderAdaptor<MB> {
	pub fn new(builder_3d: MB, uvw: Mat3) -> Self {
		PlaneMeshBuilderAdaptor {
			builder_3d,
			uvw,
		}
	}
}


impl<MB> PlaneMeshBuilderAdaptor<MB>
	where MB: PolyBuilder3D + ColoredPolyBuilder
{
	pub fn set_color(&mut self, color: impl Into<Color>) {
		self.builder_3d.set_color(color);
	}
}


impl<MB: PolyBuilder3D> PolyBuilder2D for PlaneMeshBuilderAdaptor<MB> {
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
		let vertices_3d = vs.into_iter().map(|v2| {
			self.uvw * v2.extend(1.0)
		});

		self.builder_3d.extend_3d(vertices_3d, is);
	}
}


impl<MB> ColoredPolyBuilder for PlaneMeshBuilderAdaptor<MB>
	where MB: PolyBuilder3D + ColoredPolyBuilder
{
	fn set_color(&mut self, color: impl Into<Color>) {
		self.builder_3d.set_color(color);
	}
}