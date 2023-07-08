use std::cell::Cell;


#[derive(Debug)]
pub struct BorrowState(Cell<isize>);

impl BorrowState {
	pub fn new() -> Self {
		BorrowState(Cell::new(0))
	}

	pub fn borrow(&self) {
		let new_borrow_state = self.0.get() + 1;
		assert!(new_borrow_state > 0, "tried to immutably borrow while already mutably borrowed");
		self.0.set(new_borrow_state);
	}

	pub fn unborrow(&self) {
		let new_borrow_state = self.0.get() - 1;
		assert!(new_borrow_state >= 0);
		self.0.set(new_borrow_state);
	}

	pub fn borrow_mut(&self) {
		assert!(self.0.get() == 0, "tried to mutably borrow while already borrowed");
		self.0.set(-1);
	}

	pub fn unborrow_mut(&self) {
		assert!(self.0.get() == -1);
		self.0.set(0);
	}

	pub fn is_borrowed(&self) -> bool {
		self.0.get() != 0
	}
}



