use crate::core;
use std::path::{Path, PathBuf};
use std::fmt::Debug;
use std::hash::Hash;

use std::collections::HashMap;
use anyhow::Context;

use crate::prelude::*;
use crate::upload_heap::UploadHeap;
use crate::{shaders, ImageName, SamplerName};

mod request;
pub use request::*;

mod shader;
pub use shader::*;

mod image;
pub use self::image::*;

mod framebuffer;
pub use framebuffer::*;

// Create/Destroy api for gpu resources
// Load/Cache resources from disk
// Render target/FBO/temporary image cache
//  - cache of images for use as single-frame render targets, automatically resized
//  - cache of images for use as single-frame image resources -  fixed size
//  - cache of FBOs for render passes
// Shader cache
pub struct ResourceManager {
	resource_root_path: PathBuf,

	load_shader_requests: ResourceRequestMap<LoadShaderRequest>,
	compile_shader_requests: ResourceRequestMap<CompileShaderRequest>,
	pub shaders: ResourceStorage<ShaderResource>,

	pub standard_vs_shader: ShaderHandle,
	pub fullscreen_vs_shader: ShaderHandle,
	pub flat_fs_shader: ShaderHandle,

	load_image_requests: ResourceRequestMap<LoadImageRequest>,
	create_image_requests: ResourceRequestMap<CreateImageRequest>,
	pub images: ResourceStorage<ImageResource>,

	pub blank_white_image: ImageName,
	pub blank_black_image: ImageName,

	pub nearest_sampler: SamplerName,
	pub linear_sampler: SamplerName,

	draw_pipelines: HashMap<(ShaderHandle, Option<ShaderHandle>), core::ShaderPipelineName>,
	compute_pipelines: HashMap<ShaderHandle, core::ShaderPipelineName>,

	framebuffer_cache: FramebufferCache,

	pub upload_heap: UploadHeap,

	resize_request: Option<common::Vec2i>,
}

impl ResourceManager {
	pub fn new(core: &mut core::Core, resource_root_path: &Path) -> anyhow::Result<ResourceManager> {
		let mut compile_shader_requests = ResourceRequestMap::new();
		let mut shaders = ResourceStorage::<ShaderResource>::new();

		let standard_vs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::vertex("standard vs", shaders::STANDARD_VS_SHADER_SOURCE));

		let fullscreen_vs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::vertex("fullscreen vs", shaders::FULLSCREEN_VS_SHADER_SOURCE));

		let flat_fs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::fragment("flat fs", shaders::FLAT_FS_SHADER_SOURCE));

		let blank_white_image = {
			let format = crate::ImageFormat::Rgba(crate::ComponentFormat::Unorm8);
			let image = core.create_image_2d(format, Vec2i::splat(1));
			core.upload_image(image, None, format, &[255u8, 255, 255, 255]);
			core.set_debug_label(image, "Blank white image");
			image
		};

		let blank_black_image = {
			let format = crate::ImageFormat::Rgba(crate::ComponentFormat::Unorm8);
			let image = core.create_image_2d(format, Vec2i::splat(1));
			core.upload_image(image, None, format, &[0u8, 0, 0, 255]);
			core.set_debug_label(image, "Blank black image");
			image
		};

		let nearest_sampler = {
			let sampler = core.create_sampler();
			core.set_sampler_minify_filter(sampler, crate::FilterMode::Nearest, None);
			core.set_sampler_magnify_filter(sampler, crate::FilterMode::Nearest);
			core.set_sampler_addressing_mode(sampler, crate::AddressingMode::Clamp);
			core.set_debug_label(sampler, "Nearest sampler");
			sampler
		};

		let linear_sampler = {
			let sampler = core.create_sampler();
			core.set_sampler_minify_filter(sampler, crate::FilterMode::Linear, None);
			core.set_sampler_magnify_filter(sampler, crate::FilterMode::Linear);
			core.set_sampler_addressing_mode(sampler, crate::AddressingMode::Clamp);
			core.set_debug_label(sampler, "Linear sampler");
			sampler
		};

		Ok(ResourceManager {
			resource_root_path: resource_root_path.to_owned(),

			load_shader_requests: ResourceRequestMap::new(),
			compile_shader_requests,
			shaders,

			standard_vs_shader,
			fullscreen_vs_shader,
			flat_fs_shader,

			load_image_requests: ResourceRequestMap::new(),
			create_image_requests: ResourceRequestMap::new(),
			images: ResourceStorage::new(),

			blank_white_image,
			blank_black_image,

			nearest_sampler,
			linear_sampler,

			draw_pipelines: HashMap::new(),
			compute_pipelines: HashMap::new(),

			framebuffer_cache: FramebufferCache::new(),

			upload_heap: UploadHeap::new(core),

			resize_request: None,
		})
	}

	pub fn request_resize(&mut self, new_size: common::Vec2i) {
		self.resize_request = Some(new_size);
	}

	/// Make sure all image names that will be invalidated on resize are
	/// gone before client code has a chance to ask for them.
	pub fn handle_resize(&mut self, core: &mut core::Core) {
		if let Some(_size) = self.resize_request.take() {
			// TODO(pat.m): recreate framebuffers etc
			for image in self.images.iter_mut() {
				image.on_resize(core);
			}

			self.framebuffer_cache.refresh_attachments(core, &self.images);
		}
	}

	pub fn start_frame(&mut self, core: &mut core::Core) {
		self.handle_resize(core);

		for image in self.images.iter_mut() {
			if image.clear_policy == ImageClearPolicy::DefaultAtFrameStart {
				core.clear_image_to_default(image.name);
			}
		}
	}

	/// Attempt to turn requested resources into committed GPU resources.
	pub fn process_requests(&mut self, core: &mut core::Core) -> anyhow::Result<()> {
		core.push_debug_group("Process Resource Requests");

		self.load_shader_requests.process_requests(&mut self.shaders, |def| {
			let label = def.path.display().to_string();
			let full_path = self.resource_root_path.join(&def.path);

			ShaderResource::from_disk(core, def.shader_type, &full_path, &label)
				.with_context(|| format!("Compiling shader '{}'", full_path.display()))
		})?;

		self.compile_shader_requests.process_requests(&mut self.shaders, |def| {
			ShaderResource::from_source(core, def.shader_type, &def.src, &def.label)
				.with_context(|| format!("Compiling shader '{}' from source", def.label))
		})?;

		self.load_image_requests.process_requests(&mut self.images, |def| {
			let label = def.path.display().to_string();
			let full_path = self.resource_root_path.join(&def.path);
			ImageResource::from_disk(core, &full_path, label)
				.with_context(|| format!("Loading image '{}'", full_path.display()))
		})?;

		self.create_image_requests.process_requests(&mut self.images, |def| {
			Ok(ImageResource::from_create_request(core, def))
		})?;

		// TODO(pat.m): this will never be reached if the above fails, but if the above fails
		// the whole engine is probably coming down anyway
		core.pop_debug_group();

		Ok(())
	}

	pub fn resource_path(&self) -> &Path {
		&self.resource_root_path
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

	pub fn resolve_framebuffer(&mut self, core: &core::Core, desc: impl Into<FramebufferDescription>)
		-> Option<core::FramebufferName>
	{
		self.framebuffer_cache.resolve(core, &self.images, desc.into())
	}
}


/// Request api
impl ResourceManager {
	pub fn request<R: ResourceRequest>(&mut self, request: R) -> <R::Resource as Resource>::Handle {
		request.register(self)
	}
}


pub trait ResourceHandle : Copy + Clone + Eq + PartialEq + Debug + Hash {
	fn from_raw(value: u32) -> Self;
}


// TODO(pat.m): this could be split into a generic Resource and a gfx-specific Resource with names
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

	// TODO(pat.m): make ResourceStorage generic and make this a gfx-only extension
	pub fn get_name(&self, handle: R::Handle) -> Option<R::Name> {
		self.resources.get(&handle)
			.map(R::get_name)
	}

	pub fn get_resource(&self, handle: R::Handle) -> Option<&'_ R> {
		self.resources.get(&handle)
	}

	pub fn iter(&self) -> impl Iterator<Item=&R> {
		self.resources.values()
	}

	pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut R> {
		self.resources.values_mut()
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

