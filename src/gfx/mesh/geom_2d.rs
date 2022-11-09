//! Types that implement [`BuildableGeometry2D`][BuildableGeometry2D].

use common::*;
use crate::gfx::mesh::{PolyBuilder2D, traits::BuildableGeometry2D, util::*};


#[derive(Copy, Clone, Debug)]
pub struct Quad {
	basis: Mat2x3,
}

impl Quad {
	pub fn from_matrix(basis: Mat2x3) -> Quad {
		Quad {basis}
	}

	pub fn unit() -> Quad {
		Quad::from_matrix(Mat2x3::identity())
	}

	pub fn with_size(size: Vec2) -> Quad {
		Quad::from_matrix(Mat2x3::scale(size))
	}
}

impl BuildableGeometry2D for Quad {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
		let [ux, uy, translation] = self.basis.columns();
		let (hx, hy) = (ux/2.0, uy/2.0);

		let vertices = [
			translation - hx - hy,
			translation + hx - hy,
			translation + hx + hy,
			translation - hx + hy,
		];

		mb.extend_2d(vertices, iter_fan_indices(vertices.len()));
	}
}


#[derive(Copy, Clone, Debug)]
pub struct Polygon {
	basis: Mat2x3,
	num_faces: u32,
}

impl Polygon {
	pub fn from_matrix(num_faces: u32, basis: Mat2x3) -> Polygon {
		Polygon {basis, num_faces}
	}

	pub fn unit(num_faces: u32) -> Polygon {
		Polygon::from_matrix(num_faces, Mat2x3::identity())
	}

	pub fn from_pos_scale(num_faces: u32, pos: Vec2, scale: Vec2) -> Polygon {
		Polygon::from_matrix(num_faces, Mat2x3::scale_translate(scale, pos))
	}
}

impl BuildableGeometry2D for Polygon {
	fn build<MB: PolyBuilder2D>(&self, mb: &mut MB) {
		if self.num_faces < 3 {
			return
		}

		let [ux, uy, translation] = self.basis.columns();
		let uxy = Mat2::from_columns([ux/2.0, uy/2.0]);

		let angle_increment = TAU / (self.num_faces as f32);
		let vertices = (0..self.num_faces)
			.map(|i| {
				let angle = angle_increment * i as f32 + TAU/4.0;
				translation + uxy * Vec2::from_angle(angle)
			});

		mb.extend_2d(vertices, iter_fan_indices(self.num_faces as usize));
	}
}




pub trait Transformable2: Sized {
	fn apply_transform(self, txform: Mat2x3) -> Self;

	fn scale(self, scale: Vec2) -> Self {
		self.apply_transform(Mat2x3::scale(scale))
	}

	fn uniform_scale(self, scale: f32) -> Self {
		self.apply_transform(Mat2x3::uniform_scale(scale))
	}

	fn translate(self, translation: Vec2) -> Self {
		self.apply_transform(Mat2x3::translate(translation))
	}

	fn rotate(self, rotation: f32) -> Self {
		self.apply_transform(Mat2x3::rotate(rotation))
	}
}

impl Transformable2 for Quad {
	fn apply_transform(mut self, txform: Mat2x3) -> Self {
		self.basis = txform * self.basis;
		self
	}
}

impl Transformable2 for Polygon {
	fn apply_transform(mut self, txform: Mat2x3) -> Self {
		self.basis = txform * self.basis;
		self
	}
}