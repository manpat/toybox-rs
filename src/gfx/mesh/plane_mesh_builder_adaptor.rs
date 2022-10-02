use common::*;
use crate::gfx::mesh::{PolyBuilder2D, PolyBuilder3D};

use std::ops::{Deref, DerefMut};


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

	pub fn set_xy_plane(&mut self, right: Vec3, up: Vec3) {
		self.uvw.set_column_x(right);
		self.uvw.set_column_y(up);
	}

	pub fn set_origin(&mut self, origin: Vec3) {
		self.uvw.set_column_z(origin);
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



impl<MB: PolyBuilder3D> Deref for PlaneMeshBuilderAdaptor<MB> {
	type Target = MB;
	fn deref(&self) -> &MB { &self.builder_3d }
}

impl<MB: PolyBuilder3D> DerefMut for PlaneMeshBuilderAdaptor<MB> {
	fn deref_mut(&mut self) -> &mut MB { &mut self.builder_3d }
}