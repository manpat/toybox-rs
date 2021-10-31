

/// 
pub struct IntermediateBuffer {
	samples: Vec<f32>,
	stereo: bool,
}


impl IntermediateBuffer {
	pub(in crate::audio) fn new() -> IntermediateBuffer {
		IntermediateBuffer {
			samples: Vec::new(),
			stereo: false,
		}
	}

	pub(in crate::audio) fn reformat(&mut self, samples: usize, stereo: bool) {
		let target_size = match stereo {
			false => samples,
			true => 2*samples,
		};

		self.samples.resize(target_size, 0.0);
		self.stereo = stereo;
	}

	pub fn stereo(&self) -> bool { self.stereo }
}


impl std::ops::Deref for IntermediateBuffer {
	type Target = [f32];
	fn deref(&self) -> &[f32] { &self.samples }
}


impl std::ops::DerefMut for IntermediateBuffer {
	fn deref_mut(&mut self) -> &mut [f32] { &mut self.samples }
}

