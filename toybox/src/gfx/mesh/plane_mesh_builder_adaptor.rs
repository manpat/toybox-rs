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
	pub fn new(builder_3d: MB, surface: impl Into<BuilderSurface>) -> Self {
		PlaneMeshBuilderAdaptor {
			builder_3d,
			uvw: surface.into().to_mat3(),
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





#[derive(Copy, Clone, Debug)]
pub enum OrthogonalOrientation {
	PositiveX,
	NegativeX,

	PositiveY,
	NegativeY,

	PositiveZ,
	NegativeZ,
}

impl OrthogonalOrientation {
	pub fn to_surface(self) -> BuilderSurface {
		BuilderSurface::from_orthogonal(self)
	}

	pub fn to_surface_with_origin(self, origin: Vec3) -> BuilderSurface {
		BuilderSurface::from_orthogonal(self)
			.with_origin(origin)
	}
}


#[derive(Copy, Clone, Debug)]
pub struct BuilderSurface {
	uvw: Mat3,
}

impl BuilderSurface {
	pub fn from_bases(right: Vec3, up: Vec3) -> Self {
		BuilderSurface {
			uvw: Mat3::from_columns([
				right,
				up,
				Vec3::zero(),
			])
		}
	}

	pub fn from_orthogonal(orientation: OrthogonalOrientation) -> Self {
		use OrthogonalOrientation::*;

		match orientation {
			PositiveX => Self::from_bases(Vec3::from_z(-1.0), Vec3::from_y(1.0)),
			NegativeX => Self::from_bases(Vec3::from_z(1.0), Vec3::from_y(1.0)),

			PositiveY => Self::from_bases(Vec3::from_x(1.0), Vec3::from_z(-1.0)),
			NegativeY => Self::from_bases(Vec3::from_x(1.0), Vec3::from_z(1.0)),

			PositiveZ => Self::from_bases(Vec3::from_x(1.0), Vec3::from_y(1.0)),
			NegativeZ => Self::from_bases(Vec3::from_x(-1.0), Vec3::from_y(1.0)),
		}
	}

	pub fn from_quat(quat: Quat) -> Self {
		Self::from_bases(quat.right(), quat.up())
	}

	pub fn with_origin(mut self, origin: Vec3) -> Self {
		self.uvw.set_column_z(origin);
		self
	}

	pub fn to_mat3(&self) -> Mat3 {
		self.uvw
	}
}

impl From<Mat3> for BuilderSurface {
	fn from(uvw: Mat3) -> BuilderSurface {
		BuilderSurface { uvw }
	}
}

impl From<Quat> for BuilderSurface {
	fn from(quat: Quat) -> BuilderSurface {
		BuilderSurface::from_quat(quat)
	}
}

impl From<OrthogonalOrientation> for BuilderSurface {
	fn from(orientation: OrthogonalOrientation) -> BuilderSurface {
		BuilderSurface::from_orthogonal(orientation)
	}
}