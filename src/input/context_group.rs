use crate::input::ContextID;


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContextGroupID(pub(super) usize);


/// A collection of contexts that can be enabled/disabled together.
pub struct ContextGroup {
	pub id: ContextGroupID,
	pub name: String,
}

impl ContextGroup {
	pub(crate) fn new_empty(name: String, id: ContextGroupID) -> ContextGroup {
		ContextGroup {
			id, name,
		}
	}
}