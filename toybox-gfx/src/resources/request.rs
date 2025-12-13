use super::*;


pub trait ResourceRequest : PartialEq + Eq + Hash {
	type Resource : Resource;

	fn register(self, rm: &mut Resources) -> <Self::Resource as Resource>::Handle;

	// fn process(&self, ctx: &mut ResourceRequestContext<'_, '_>) -> 
}




#[derive(Debug)]
pub struct ResourceRequestMap<Request>
	where Request: ResourceRequest
{
	request_to_handle: HashMap<Request, <Request::Resource as Resource>::Handle>,
	requests: HashMap<Request, <Request::Resource as Resource>::Handle>,
}

impl<Request> ResourceRequestMap<Request>
	where Request: ResourceRequest
{
	pub(crate) fn new() -> Self {
		ResourceRequestMap {
			request_to_handle: HashMap::new(),
			requests: HashMap::new(),
		}
	}

	pub fn get_handle(&self, request: &Request) -> Option<<Request::Resource as Resource>::Handle> {
		self.request_to_handle.get(request).cloned()
	}

	pub fn request_handle(&mut self, storage: &mut ResourceStorage<Request::Resource>, request: Request) -> <Request::Resource as Resource>::Handle {
		if let Some(handle) = self.get_handle(&request) {
			return handle
		}

		*self.requests.entry(request)
			.or_insert_with(|| storage.new_handle())
	}

	pub(crate) fn process_requests<F>(&mut self, storage: &mut ResourceStorage<Request::Resource>, mut f: F) -> anyhow::Result<()>
		where F: FnMut(&Request) -> anyhow::Result<Request::Resource>
	{
		for (request, handle) in self.requests.drain() {
			let resource = f(&request)?;
			storage.insert(handle, resource);
			self.request_to_handle.insert(request, handle);
		}

		Ok(())
	}
}


// pub struct ResourceRequestContext<'core, 'rm> {
// 	pub core: &'core mut Core,
// 	pub resource_path: &'rm Path,
// }