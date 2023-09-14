use crate::core;
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use std::hash::Hash;

use std::collections::HashMap;
use anyhow::Context;

use crate::upload_heap::UploadHeap;

pub mod shader;

pub use shader::*;

// Create/Destroy api for gpu resources
// Load/Cache resources from disk
// Render target/FBO/temporary image cache
//  - cache of images for use as single-frame render targets, automatically resized
//  - cache of images for use as single-frame image resources -  fixed size
//  - cache of FBOs for render passes
// Shader cache
pub struct ResourceManager {
	resource_root_path: PathBuf,

	load_shader_requests: ResourceRequestMap<LoadShaderRequest, shader::ShaderResource>,
	compile_shader_requests: ResourceRequestMap<CompileShaderRequest, shader::ShaderResource>,
	pub shaders: ResourceStorage<shader::ShaderResource>,

	draw_pipelines: HashMap<(ShaderHandle, Option<ShaderHandle>), core::ShaderPipelineName>,
	compute_pipelines: HashMap<ShaderHandle, core::ShaderPipelineName>,

	pub upload_heap: UploadHeap,

	resize_request: Option<common::Vec2i>,
}

impl ResourceManager {
	pub fn new(core: &mut core::Core) -> ResourceManager {
		let resource_root_path = PathBuf::from("resource");
		if !resource_root_path.exists() {
			panic!("Can't find resource directory - make sure to run from correct working directory!");
		}

		ResourceManager {
			resource_root_path,

			load_shader_requests: ResourceRequestMap::new(),
			compile_shader_requests: ResourceRequestMap::new(),
			shaders: ResourceStorage::new(),

			draw_pipelines: HashMap::new(),
			compute_pipelines: HashMap::new(),

			upload_heap: UploadHeap::new(core),

			resize_request: None,
		}
	}

	pub fn request_resize(&mut self, new_size: common::Vec2i) {
		self.resize_request = Some(new_size);
	}

	/// Attempt to turn requested resources into committed GPU resources.
	pub fn process_requests(&mut self, core: &mut core::Core) -> anyhow::Result<()> {
		core.push_debug_group("Process Resource Requests");

		if let Some(_size) = self.resize_request.take() {
			// TODO(pat.m): Resize textures, recreate framebuffers etc
			println!("RESIZE {_size:?}");
		}

		self.load_shader_requests.process_requests(&mut self.shaders, |def| {
			let full_path = self.resource_root_path.join(&def.path);

			shader::ShaderResource::from_disk(core, def.shader_type, &full_path)
				.with_context(|| format!("Compiling shader '{}'", full_path.display()))
		})?;

		self.compile_shader_requests.process_requests(&mut self.shaders, |def| {
			shader::ShaderResource::from_source(core, def.shader_type, &def.src, &def.label)
				.with_context(|| format!("Compiling shader '{}' from source", def.label))
		})?;

		// TODO(pat.m): this will never be reached if the above fails, but if the above fails
		// the whole engine is probably coming down anyway
		core.pop_debug_group();

		Ok(())
	}
}

/// Execution api
impl ResourceManager {
	pub fn resolve_draw_pipeline(&mut self, core: &mut core::Core,
		vertex_shader: shader::ShaderHandle, fragment_shader: impl Into<Option<shader::ShaderHandle>>)
		-> crate::core::ShaderPipelineName
	{
		let fragment_shader = fragment_shader.into();
		let key = (vertex_shader, fragment_shader);

		if let Some(&name) = self.draw_pipelines.get(&key) {
			return name;
		}

		let pipeline = core.create_shader_pipeline();

		let vertex_shader_name = self.shaders.get_name(vertex_shader).unwrap();
		core.attach_shader_to_pipeline(pipeline, vertex_shader_name);

		if let Some(fragment_shader) = fragment_shader {
			let fragment_shader_name = self.shaders.get_name(fragment_shader).unwrap();
			core.attach_shader_to_pipeline(pipeline, fragment_shader_name);
		}

		core.set_debug_label(pipeline, "draw pipeline");

		self.draw_pipelines.insert(key, pipeline);

		pipeline
	}

	pub fn resolve_compute_pipeline(&mut self, core: &mut core::Core, compute_shader: shader::ShaderHandle)
		-> crate::core::ShaderPipelineName
	{
		if let Some(&name) = self.compute_pipelines.get(&compute_shader) {
			return name;
		}

		let pipeline = core.create_shader_pipeline();
		let compute_shader_name = self.shaders.get_name(compute_shader).unwrap();
		core.attach_shader_to_pipeline(pipeline, compute_shader_name);
		core.set_debug_label(pipeline, "compute pipeline");

		self.compute_pipelines.insert(compute_shader, pipeline);

		pipeline
	}
}


/// Request api
impl ResourceManager {
	// TODO(pat.m): these could literally just be replaced with a single request(Request)
	pub fn create_shader(&mut self, def: LoadShaderRequest) -> shader::ShaderHandle {
		self.load_shader_requests.request_handle(&mut self.shaders, def)
	}

	pub fn compile_shader(&mut self, def: CompileShaderRequest) -> shader::ShaderHandle {
		self.compile_shader_requests.request_handle(&mut self.shaders, def)
	}
}


pub trait ResourceHandle : Copy + Clone + Eq + PartialEq + Debug + Hash {
	fn from_raw(value: u32) -> Self;
}


pub trait Resource : Debug {
	type Handle : ResourceHandle;
	type Name : core::ResourceName;

	// TODO(pat.m): ref counting?
	fn get_name(&self) -> Self::Name;
}


#[derive(Debug)]
pub struct ResourceStorage<R: Resource> {
	resources: HashMap<R::Handle, R>,
	handle_counter: u32,
}

impl<R: Resource> ResourceStorage<R> {
	fn new() -> Self {
		ResourceStorage {
			resources: HashMap::new(),
			handle_counter: 0,
		}
	}
	pub fn get_name(&self, handle: R::Handle) -> Option<R::Name> {
		self.resources.get(&handle)
			.map(R::get_name)
	}

	pub fn get_resource(&self, handle: R::Handle) -> Option<&'_ R> {
		self.resources.get(&handle)
	}

	fn insert(&mut self, handle: R::Handle, resource: R) {
		self.resources.insert(handle, resource);
	}

	fn new_handle(&mut self) -> R::Handle {
		let value = self.handle_counter;
		self.handle_counter += 1;
		R::Handle::from_raw(value)
	}
}


#[derive(Debug)]
pub struct ResourceRequestMap<Request, R: Resource>
	where Request: PartialEq + Eq + Hash
{
	request_to_handle: HashMap<Request, R::Handle>,
	requests: HashMap<Request, R::Handle>,
}

impl<Request, R: Resource> ResourceRequestMap<Request, R>
	where Request: PartialEq + Eq + Hash
{
	fn new() -> Self {
		ResourceRequestMap {
			request_to_handle: HashMap::new(),
			requests: HashMap::new(),
		}
	}

	pub fn get_handle(&self, request: &Request) -> Option<R::Handle> {
		self.request_to_handle.get(request).cloned()
	}

	pub fn request_handle(&mut self, storage: &mut ResourceStorage<R>, request: Request) -> R::Handle {
		if let Some(handle) = self.get_handle(&request) {
			return handle
		}

		*self.requests.entry(request)
			.or_insert_with(|| storage.new_handle())
	}

	fn process_requests<F>(&mut self, storage: &mut ResourceStorage<R>, mut f: F) -> anyhow::Result<()>
		where F: FnMut(&Request) -> anyhow::Result<R>
	{
		for (request, handle) in self.requests.drain() {
			let resource = f(&request)?;
			storage.insert(handle, resource);
			self.request_to_handle.insert(request, handle);
		}

		Ok(())
	}
}