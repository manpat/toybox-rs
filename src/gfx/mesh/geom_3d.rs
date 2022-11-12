//! Types that implement [`BuildableGeometry3D`][BuildableGeometry3D].

use common::*;
use crate::gfx::mesh::{PolyBuilder3D, traits::BuildableGeometry3D};

pub struct Tetrahedron {
	basis: Mat3x4,
}

impl Tetrahedron {
	pub fn from_matrix(basis: Mat3x4) -> Tetrahedron {
		Tetrahedron {basis}
	}

	pub fn unit() -> Tetrahedron {
		Tetrahedron::from_matrix(Mat3x4::identity())
	}
}

impl BuildableGeometry3D for Tetrahedron {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
		let [ux, uy, uz, translation] = self.basis.columns();

		let verts = [
			translation + ux,
			translation + ux*(TAU/3.0).cos() - uz*(TAU/3.0).sin(),
			translation + ux*(TAU/3.0).cos() + uz*(TAU/3.0).sin(),
			translation + uy,
		];

		let indices = [
			0, 2, 1,

			3, 0, 1,
			3, 1, 2,
			3, 2, 0,
		];

		mb.extend_3d(verts, indices);
	}
}


pub struct Cuboid {
	basis: Mat3x4,
}

impl Cuboid {
	pub fn from_matrix(basis: Mat3x4) -> Cuboid {
		Cuboid {basis}
	}

	pub fn unit() -> Cuboid {
		Cuboid::from_matrix(Mat3x4::identity())
	}

	pub fn with_size(size: Vec3) -> Cuboid {
		Cuboid::from_matrix(Mat3x4::scale(size))
	}

	pub fn from_points(Vec3{x: ax, y: ay, z: az}: Vec3, Vec3{x: bx, y: by, z: bz}: Vec3) -> Cuboid {
		let min = Vec3::new(ax.min(bx), ay.min(by), az.min(bz));
		let max = Vec3::new(ax.max(bx), ay.max(by), az.max(bz));

		let size = max - min;
		let center = min + size / 2.0;
		Cuboid::from_matrix(Mat3x4::scale_translate(size, center))
	}
}

impl BuildableGeometry3D for Cuboid {
	fn build<MB: PolyBuilder3D>(&self, mb: &mut MB) {
		let [ux, uy, uz, translation] = self.basis.columns();
		let (hx, hy, hz) = (ux/2.0, uy/2.0, uz/2.0);

		let verts = [
			translation - hx - hy - hz,
			translation - hx + hy - hz,
			translation + hx + hy - hz,
			translation + hx - hy - hz,

			translation - hx - hy + hz,
			translation - hx + hy + hz,
			translation + hx + hy + hz,
			translation + hx - hy + hz,
		];

		let indices = [
			// -Z, +Z
			3, 0, 1,  3, 1, 2,
			4, 7, 6,  4, 6, 5,

			// -X, +X
			0, 4, 5,  0, 5, 1,
			7, 3, 2,  7, 2, 6,

			// -Y, +Y
			0, 3, 7,  0, 7, 4,
			5, 6, 2,  5, 2, 1,
		];

		mb.extend_3d(verts, indices);
	}
}






pub trait Transformable3: Sized {
	fn apply_transform(self, txform: Mat3x4) -> Self;

	fn scale(self, scale: Vec3) -> Self {
		self.apply_transform(Mat3x4::scale(scale))
	}

	fn uniform_scale(self, scale: f32) -> Self {
		self.apply_transform(Mat3x4::uniform_scale(scale))
	}

	fn translate(self, translation: Vec3) -> Self {
		self.apply_transform(Mat3x4::translate(translation))
	}

	fn rotate_x(self, rotation: f32) -> Self {
		self.apply_transform(Mat3x4::rotate_x(rotation))
	}

	fn rotate_y(self, rotation: f32) -> Self {
		self.apply_transform(Mat3x4::rotate_y(rotation))
	}

	fn rotate_z(self, rotation: f32) -> Self {
		self.apply_transform(Mat3x4::rotate_z(rotation))
	}
}

impl Transformable3 for Tetrahedron {
	fn apply_transform(mut self, txform: Mat3x4) -> Self {
		self.basis = txform * self.basis;
		self
	}
}

impl Transformable3 for Cuboid {
	fn apply_transform(mut self, txform: Mat3x4) -> Self {
		self.basis = txform * self.basis;
		self
	}
}