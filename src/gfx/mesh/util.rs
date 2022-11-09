use crate::prelude::*;
use itertools::Either;


pub fn iter_fan_indices(num_vertices: usize) -> impl Iterator<Item=u16> {
	if num_vertices < 3 {
		return Either::Left(std::iter::empty());
	}

	// Silently truncate indices.
	let num_vertices = num_vertices.min(u16::MAX as usize - 2) as u16;

	let indices = (0..num_vertices-2)
		.flat_map(|i| [0, i+1, i+2]);

	Either::Right(indices)
}


pub fn iter_closed_fan_indices(num_vertices: usize) -> impl Iterator<Item=u16> {
	if num_vertices < 3 {
		return Either::Left(std::iter::empty());
	}

	// Silently truncate indices.
	let num_vertices = num_vertices.min(u16::MAX as usize - 2) as u16;

	let indices = (0..num_vertices-2)
		.flat_map(|i| [0, i+1, i+2])
		.chain([0, num_vertices-1, 1]);

	Either::Right(indices)
}