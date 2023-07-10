use std::rc::{Rc, Weak};
use std::collections::HashMap;
use crate::utility;


/// Unique identifier for a `ResourceScope`.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceScopeID(pub(crate) usize);



/// A reference counted token associated with a `ResourceScope`.
/// Allows [`System`] to track when a `ResourceScope` is no longer needed.
#[derive(Clone, Debug)]
pub struct ResourceScopeToken {
	ref_count: Rc<()>,
	id: ResourceScopeID,
}

impl ResourceScopeToken {
	fn new(id: ResourceScopeID) -> ResourceScopeToken {
		ResourceScopeToken {
			ref_count: Rc::new(()),
			id
		}
	}

	pub fn id(&self) -> ResourceScopeID {
		self.id
	}

	pub fn to_weak(&self) -> ResourceScopeTokenWeak {
		ResourceScopeTokenWeak {
			ref_count: Rc::downgrade(&self.ref_count),
			id: self.id,
		}
	}
}

impl From<&'_ ResourceScopeToken> for Option<ResourceScopeID> {
	fn from(token: &ResourceScopeToken) -> Option<ResourceScopeID> {
		Some(token.id)
	}
}




#[derive(Clone, Debug)]
pub struct ResourceScopeTokenWeak {
	ref_count: Weak<()>,
	id: ResourceScopeID,
}

impl ResourceScopeTokenWeak {
	pub fn is_alive(&self) -> bool {
		self.ref_count.strong_count() > 0
	}

	pub fn id(&self) -> ResourceScopeID {
		self.id
	}
}




pub struct ResourceScopeAllocator {
	id_counter: utility::IdCounter,
	global_scope_token: ResourceScopeToken,

	scopes: Vec<ResourceScopeTokenWeak>,
}


impl ResourceScopeAllocator {
	pub fn new() -> ResourceScopeAllocator {
		let mut id_counter = utility::IdCounter::new();

		let global_scope_id = ResourceScopeID(id_counter.next());
		let global_scope_token = ResourceScopeToken::new(global_scope_id);

		ResourceScopeAllocator {
			id_counter,
			global_scope_token,
			scopes: Vec::new(),
		}
	}
	
	/// Create a new `ResourceScope` and tie its lifetime to the returned [`ResourceScopeToken`].
	/// The returned token can be passed around and cloned freely. Once no instances of the returned token remain alive,
	/// the [`System`] will destroy all resources that were associated with the resource scope during its lifetime.
	pub fn new_resource_scope(&mut self) -> ResourceScopeToken {
		let resource_scope_id = ResourceScopeID(self.id_counter.next());
		let resource_scope_token = ResourceScopeToken::new(resource_scope_id);
		self.scopes.push(resource_scope_token.to_weak());
		resource_scope_token
	}

	pub fn global_scope_token(&self) -> ResourceScopeToken {
		self.global_scope_token.clone()
	}

	pub fn reap_dead_scopes(&mut self) -> Vec<ResourceScopeID> {
		let to_remove: Vec<_> = self.scopes.iter()
			.filter(|token| !token.is_alive())
			.map(|token| token.id)
			.collect();

		self.scopes.retain(|token| !to_remove.contains(&token.id));

		to_remove
	}
}



pub struct ResourceScopeStore<R: ScopedResource> {
	resource_scopes: HashMap<ResourceScopeID, ResourceScope<R>>,
	global_scope_id: ResourceScopeID,
}


impl<R: ScopedResource> ResourceScopeStore<R> {
	pub fn new(global_scope_token: ResourceScopeToken) -> Self {
		let global_scope_id = global_scope_token.id();
		let global_resource_scope = ResourceScope::new(global_scope_token);

		ResourceScopeStore {
			resource_scopes: [(global_scope_id, global_resource_scope)].into(),
			global_scope_id,
		}
	}
	
	pub fn register_scope(&mut self, token: ResourceScopeToken) {
		let token_id = token.id();
		self.resource_scopes.insert(token_id, ResourceScope::new(token));
	}
	
	pub fn cleanup_scope(&mut self, scope_id: ResourceScopeID, mut context: R::Context<'_>) {
		let mut scope = self.resource_scopes.remove(&scope_id)
			.expect("Tried to access already freed scope group");

		scope.destroy_owned_resources(&mut context);
	}

	pub fn cleanup_all(&mut self, mut context: R::Context<'_>) {
		for (_, scope) in self.resource_scopes.iter_mut() {
			scope.destroy_owned_resources(&mut context);
		}
	}

	pub fn get_mut(&mut self, resource_scope_id: impl Into<Option<ResourceScopeID>>) -> &mut ResourceScope<R> {
		let resource_scope_id = resource_scope_id.into()
			.unwrap_or(self.global_scope_id);

		self.resource_scopes.get_mut(&resource_scope_id)
			.expect("Tried to access already freed scope group")
	}
}





/// Keeps track of all resources which should be destroyed when all associated [`ResourceScopeTokens`][ResourceScopeToken]
/// are dropped.
///
/// ## Note
/// It is up to the owner to actually trigger cleanup - dropping a [`ResourceScope`] does nothing.
pub struct ResourceScope<R: ScopedResource> {
	resources: Vec<R>,
	token: ResourceScopeTokenWeak,
}

impl<R: ScopedResource> ResourceScope<R> {
	pub fn new(token: ResourceScopeToken) -> Self {
		ResourceScope {
			resources: Vec::new(),
			token: token.to_weak()
		}
	}

	pub fn id(&self) -> ResourceScopeID {
		self.token.id()
	}

	pub fn insert(&mut self, handle: R) {
		self.resources.push(handle);
	}

	pub fn destroy_owned_resources(&mut self, context: &mut R::Context<'_>) {
		for resource in self.resources.drain(..) {
			resource.destroy(context);
		}

		self.resources.clear();
	}
}

impl<R: ScopedResource> std::ops::Drop for ResourceScope<R> {
	fn drop(&mut self) {
		assert!(self.resources.is_empty(), "ResourceScope has been dropped without being cleaned up!");
	}
}



pub trait ScopedResource : std::fmt::Debug {
	type Context<'c>;

	fn destroy(self, context: &mut Self::Context<'_>);
}

