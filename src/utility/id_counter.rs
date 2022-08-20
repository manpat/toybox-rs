

/// Simple utility for generating unique (for long enough) values for new Ids.
pub struct IdCounter(usize);

impl IdCounter {
	pub fn new() -> IdCounter {
		IdCounter(0)
	}

	pub fn with_initial(initial: usize) -> IdCounter {
		IdCounter(initial)
	}

	pub fn next(&mut self) -> usize {
		let next_id = self.0 + 1;
		std::mem::replace(&mut self.0, next_id)
	}
}