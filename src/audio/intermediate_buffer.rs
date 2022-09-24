use std::simd::Simd;



const LANE_COUNT: usize = 8;


/// 
pub struct IntermediateBuffer {
	samples: Vec<Simd<f32, LANE_COUNT>>,
	stereo: bool,
}


impl IntermediateBuffer {
	pub(in crate::audio) fn new(sample_count: usize, stereo: bool) -> IntermediateBuffer {
		let target_size = match stereo {
			false => sample_count,
			true => 2*sample_count,
		};

		assert!(target_size % LANE_COUNT == 0);

		IntermediateBuffer {
			samples: vec![Simd::splat(0.0); target_size / LANE_COUNT],
			stereo,
		}
	}

	pub fn stereo(&self) -> bool { self.stereo }

	pub fn as_simd(&self) -> &[Simd<f32, LANE_COUNT>] { &self.samples }

	pub fn as_simd_mut(&mut self) -> &mut [Simd<f32, LANE_COUNT>] { &mut self.samples }
}


impl std::ops::Deref for IntermediateBuffer {
	type Target = [f32];
	fn deref(&self) -> &[f32] { simd_slice_to_slice(&self.samples) }
}


impl std::ops::DerefMut for IntermediateBuffer {
	fn deref_mut(&mut self) -> &mut [f32] { simd_slice_to_slice_mut(&mut self.samples) }
}


impl<'a> std::iter::IntoIterator for &'a IntermediateBuffer {
    type Item = &'a f32;
    type IntoIter = std::slice::Iter<'a, f32>;

    fn into_iter(self) -> std::slice::Iter<'a, f32> {
        simd_slice_to_slice(&self.samples).iter()
    }
}


impl<'a> std::iter::IntoIterator for &'a mut IntermediateBuffer {
    type Item = &'a mut f32;
    type IntoIter = std::slice::IterMut<'a, f32>;

    fn into_iter(self) -> std::slice::IterMut<'a, f32> {
        simd_slice_to_slice_mut(&mut self.samples).iter_mut()
    }
}




fn simd_slice_to_slice<T, const N: usize>(slice: &[Simd<T, N>]) -> &[T]
	where T: std::simd::SimdElement
		, std::simd::LaneCount<N>: std::simd::SupportedLaneCount
{
	unsafe {
		// SAFETY: It is sound to transmute Simd<T, N> -> [T; N], as they have the same layout.
		// Alignment requirements are less strict for arrays than for Simd.
		std::slice::from_raw_parts(
			slice.as_ptr() as *const T,
			slice.len() * N
		)
	}
}

fn simd_slice_to_slice_mut<T, const N: usize>(slice: &mut [Simd<T, N>]) -> &mut [T]
	where T: std::simd::SimdElement
		, std::simd::LaneCount<N>: std::simd::SupportedLaneCount
{
	unsafe {
		// SAFETY: It is sound to transmute Simd<T, N> -> [T; N], as they have the same layout.
		// Alignment requirements are less strict for arrays than for Simd.
		std::slice::from_raw_parts_mut(
			slice.as_mut_ptr() as *mut T,
			slice.len() * N
		)
	}
}