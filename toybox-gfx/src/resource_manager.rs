use crate::core;
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use std::hash::Hash;

use std::collections::HashMap;
use anyhow::Context;

use crate::upload_heap::UploadHeap;

pub mod shader;


// Create/Destroy api for gpu resources
// Load/Cache resources from disk
// Render target/FBO/temporary image cahage
//  - cache of images for use as single-frame render targets, automatically resized
//  - cache of images for use as single-frame image resources -  fixed size
//  - cache of FBOs for render passes
// Shader cache
pub struct ResourceManager {
	resource_root_path: PathBuf,

	pub shaders: ResourceStorage<shader::ShaderResource>,

	global_pipeline: crate::core::ShaderPipelineName,
	pub global_vao: crate::core::VaoName,

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
			shaders: ResourceStorage::new(),

			global_pipeline: core.create_shader_pipeline(),
			global_vao: core.create_vao(),

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

		self.shaders.process_requests(|def| {
			let full_path = self.resource_root_path.join(&def.path);

			shader::ShaderResource::from_disk(core, def.shader_type, &full_path)
				.with_context(|| format!("Compiling shader '{}'", full_path.display()))
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
		if let Some(fragment_shader) = fragment_shader.into() {
			let fragment_shader_name = self.shaders.get_name(fragment_shader).unwrap();
			core.attach_shader_to_pipeline(self.global_pipeline, fragment_shader_name);
		} else {
			// Only clear if we're removing the fragment stage
			core.clear_shader_pipeline(self.global_pipeline);
		}

		let vertex_shader_name = self.shaders.get_name(vertex_shader).unwrap();
		core.attach_shader_to_pipeline(self.global_pipeline, vertex_shader_name);

		self.global_pipeline
	}

	pub fn resolve_compute_pipeline(&mut self, core: &mut core::Core, compute_shader: shader::ShaderHandle)
		-> crate::core::ShaderPipelineName
	{
		// core.clear_shader_pipeline(self.global_pipeline);

		let compute_shader_name = self.shaders.get_name(compute_shader).unwrap();
		core.attach_shader_to_pipeline(self.global_pipeline, compute_shader_name);

		self.global_pipeline
	}
}


/// Request api
impl ResourceManager {
	pub fn create_shader(&mut self, def: shader::ShaderDef) -> shader::ShaderHandle {
		self.shaders.get_or_request_handle(def)
	}
}


pub trait ResourceHandle : Copy + Clone + Eq + PartialEq + Debug + Hash {
	fn from_raw(value: u32) -> Self;
}


pub trait Resource : Debug {
	type Handle : ResourceHandle;
	type Name : core::ResourceName;
	type Def : PartialEq + Eq + Hash;

	// TODO(pat.m): ref counting?
	fn get_name(&self) -> Self::Name;
}


#[derive(Debug)]
pub struct ResourceStorage<R: Resource> {
	handle_counter: u32,
	resources: HashMap<R::Handle, R>,
	def_to_handle: HashMap<R::Def, R::Handle>,
	requests: HashMap<R::Def, R::Handle>,
}

impl<R: Resource> ResourceStorage<R> {
	fn new() -> Self {
		ResourceStorage {
			handle_counter: 0,
			resources: HashMap::new(),
			def_to_handle: HashMap::new(),
			requests: HashMap::new(),
		}
	}

	pub fn get_handle(&self, def: &R::Def) -> Option<R::Handle> {
		self.def_to_handle.get(def).cloned()
	}

	pub fn get_name(&self, handle: R::Handle) -> Option<R::Name> {
		self.resources.get(&handle)
			.map(R::get_name)
	}

	pub fn get_or_request_handle(&mut self, def: R::Def) -> R::Handle {
		if let Some(handle) = self.get_handle(&def) {
			return handle
		}

		*self.requests.entry(def)
			.or_insert_with(|| {
				let value = self.handle_counter;
				self.handle_counter += 1;
				R::Handle::from_raw(value)
			})
	}

	fn process_requests<F>(&mut self, mut f: F) -> anyhow::Result<()>
		where F: FnMut(&R::Def) -> anyhow::Result<R>
	{
		for (def, handle) in self.requests.drain() {
			let resource = f(&def)?;
			self.resources.insert(handle, resource);
			self.def_to_handle.insert(def, handle);
		}

		Ok(())
	}
}