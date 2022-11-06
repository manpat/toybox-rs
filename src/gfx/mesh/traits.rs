use common::*;
use crate::gfx::mesh::{PlaneMeshBuilderAdaptor, BuilderSurface};


/// An interface for types capable of constructing geometry from [`Vec2s`][Vec2] and (by extension)
/// types implementing [`BuildableGeometry2D`].
pub trait PolyBuilder2D {
	/// Given an iterator of 2D positions and indices into that stream, append the described geometry
	/// to the internal geometry buffer (whatever that might be).
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>);

	/// Given some object implementing [`BuildableGeometry2D`], append the geometry it describes
	/// to the internal geometry buffer.
	fn build(&mut self, geom: impl BuildableGeometry2D) where Self: Sized {
		geom.build(self)
	}

	/// Build some geometry from an iterator of vertices of length `num_vertices`.
	/// Generates triangles between the first vertex and every subsequent pair of vertices, forming a 'fan'.
	fn extend_2d_fan(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec2>) {
		if num_vertices < 3 {
			return
		}

		let indices = (0..num_vertices as u16-2)
			.flat_map(|i| [0, i+1, i+2]);

		self.extend_2d(vs, indices);
	}

	/// Build some geometry from an iterator of vertices of length `num_vertices`.
	/// Generates triangles between the first vertex and every subsequent pair of vertices, forming a 'fan'.
	/// Different from [`PolyBuilder2D::extend_2d_fan`] in that this function duplicates the second vertex.
	/// This is useful for geometry where the first vertex lies at some kind of 'center'.
	fn extend_2d_fan_closed(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec2>) {
		if num_vertices < 3 {
			return
		}

		let indices = (0..num_vertices as u16-2)
			.flat_map(|i| [0, i+1, i+2])
			.chain([0, num_vertices as u16-1, 1]);

		self.extend_2d(vs, indices);
	}
}

impl<PB: PolyBuilder2D> PolyBuilder2D for &mut PB {
	fn extend_2d(&mut self, vs: impl IntoIterator<Item=Vec2>, is: impl IntoIterator<Item=u16>) {
		(*self).extend_2d(vs, is);
	}
}


/// An interface for types capable of constructing geometry from [`Vec3s`][Vec3] and (by extension)
/// types implementing [`BuildableGeometry3D`].
pub trait PolyBuilder3D {
	/// Given an iterator of 3D positions and indices into that stream, append the described geometry
	/// to the owned geometry buffer (whatever that might be).
	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>);

	/// Given some object implementing [`BuildableGeometry3D`], append the geometry it describes
	/// to the internal geometry buffer.
	fn build(&mut self, geom: impl BuildableGeometry3D) where Self: Sized {
		geom.build(self)
	}

	/// Construct a type implementing [`PolyBuilder2D`] from this poly builder, given a 'plane' to build
	/// new 2D geometry onto - defined by `surface`.
	/// See [`PlaneMeshBuilderAdaptor`] for more information.
	fn on_plane(self, surface: impl Into<BuilderSurface>) -> PlaneMeshBuilderAdaptor<Self> where Self: Sized {
		PlaneMeshBuilderAdaptor::new(self, surface)
	}
	
	/// Same as [`PolyBuilder3D::on_plane`] but doesn't take ownership.
	fn on_plane_ref(&mut self, surface: impl Into<BuilderSurface>) -> PlaneMeshBuilderAdaptor<&'_ mut Self> where Self: Sized {
		PlaneMeshBuilderAdaptor::new(self, surface)
	}

	/// Build some geometry from an iterator of vertices of length `num_vertices`.
	/// Generates triangles between the first vertex and every subsequent pair of vertices, forming a 'fan'.
	fn extend_3d_fan(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec3>) {
		if num_vertices < 3 {
			return
		}

		let indices = (0..num_vertices as u16-2)
			.flat_map(|i| [0, i+1, i+2]);

		self.extend_3d(vs, indices);
	}

	/// Build some geometry from an iterator of vertices of length `num_vertices`.
	/// Generates triangles between the first vertex and every subsequent pair of vertices, forming a 'fan'.
	/// Different from [`PolyBuilder3D::extend_3d_fan`] in that this function duplicates the second vertex.
	/// This is useful for geometry where the first vertex lies at some kind of 'center'.
	fn extend_3d_fan_closed(&mut self, num_vertices: u32, vs: impl IntoIterator<Item=Vec3>) {
		if num_vertices < 3 {
			return
		}

		let indices = (0..num_vertices as u16-2)
			.flat_map(|i| [0, i+1, i+2])
			.chain([0, num_vertices as u16-1, 1]);

		self.extend_3d(vs, indices);
	}
}

impl<PB: PolyBuilder3D> PolyBuilder3D for &mut PB {
	fn extend_3d(&mut self, vs: impl IntoIterator<Item=Vec3>, is: impl IntoIterator<Item=u16>) {
		(*self).extend_3d(vs, is);
	}
}



/// A type representing some 2D geometry that can be built into a type implementing [`PolyBuilder2D`].
pub trait BuildableGeometry2D {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB);
}

/// A type representing some 3D geometry that can be built into a type implementing [`PolyBuilder3D`].
pub trait BuildableGeometry3D {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB);
}


impl<G: BuildableGeometry2D> BuildableGeometry2D for &G {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
		(*self).build(mb);
	}
}

impl<G: BuildableGeometry3D> BuildableGeometry3D for &G {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
		(*self).build(mb);
	}
}


