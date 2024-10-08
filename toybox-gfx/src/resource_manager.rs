use crate::core;
use std::fmt::Debug;
use std::hash::Hash;

use std::collections::HashMap;
use anyhow::Context;
use tracing::instrument;

use crate::prelude::*;
use crate::upload_heap::UploadHeap;
use crate::{shaders, ImageName, SamplerName};

pub mod arguments;
pub use arguments::*;

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
	load_shader_requests: ResourceRequestMap<LoadShaderRequest>,
	compile_shader_requests: ResourceRequestMap<CompileShaderRequest>,
	pub shaders: ResourceStorage<ShaderResource>,

	load_image_requests: ResourceRequestMap<LoadImageRequest>,
	load_image_array_requests: ResourceRequestMap<LoadImageArrayRequest>,
	create_image_requests: ResourceRequestMap<CreateImageRequest>,
	pub images: ResourceStorage<ImageResource>,

	standard_vs_shader: ShaderHandle,
	fullscreen_vs_shader: ShaderHandle,
	flat_textured_fs_shader: ShaderHandle,

	blank_white_image: ImageName,
	blank_black_image: ImageName,

	nearest_sampler: SamplerName,
	linear_sampler: SamplerName,

	nearest_sampler_repeat: SamplerName,
	linear_sampler_repeat: SamplerName,

	draw_pipelines: HashMap<(ShaderHandle, Option<ShaderHandle>), core::ShaderPipelineName>,
	compute_pipelines: HashMap<ShaderHandle, core::ShaderPipelineName>,

	framebuffer_cache: FramebufferCache,

	pub upload_heap: UploadHeap,

	resize_request: Option<common::Vec2i>,
}

impl ResourceManager {
	pub fn new(core: &mut core::Core) -> anyhow::Result<ResourceManager> {
		let mut compile_shader_requests = ResourceRequestMap::new();
		let mut shaders = ResourceStorage::<ShaderResource>::new();

		let standard_vs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::vertex("standard vs", shaders::STANDARD_VS_SHADER_SOURCE));

		let fullscreen_vs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::vertex("fullscreen vs", shaders::FULLSCREEN_VS_SHADER_SOURCE));

		let flat_textured_fs_shader = compile_shader_requests.request_handle(&mut shaders,
			CompileShaderRequest::fragment("flat textured fs", shaders::FLAT_TEXTURED_FS_SHADER_SOURCE));

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

		let nearest_sampler_repeat = {
			let sampler = core.create_sampler();
			core.set_sampler_minify_filter(sampler, crate::FilterMode::Nearest, None);
			core.set_sampler_magnify_filter(sampler, crate::FilterMode::Nearest);
			core.set_sampler_addressing_mode(sampler, crate::AddressingMode::Repeat);
			core.set_debug_label(sampler, "Nearest repeating sampler");
			sampler
		};

		let linear_sampler_repeat = {
			let sampler = core.create_sampler();
			core.set_sampler_minify_filter(sampler, crate::FilterMode::Linear, None);
			core.set_sampler_magnify_filter(sampler, crate::FilterMode::Linear);
			core.set_sampler_addressing_mode(sampler, crate::AddressingMode::Repeat);
			core.set_debug_label(sampler, "Linear repeating sampler");
			sampler
		};

		Ok(ResourceManager {
			load_shader_requests: ResourceRequestMap::new(),
			compile_shader_requests,
			shaders,

			load_image_requests: ResourceRequestMap::new(),
			load_image_array_requests: ResourceRequestMap::new(),
			create_image_requests: ResourceRequestMap::new(),
			images: ResourceStorage::new(),

			standard_vs_shader,
			fullscreen_vs_shader,
			flat_textured_fs_shader,

			blank_white_image,
			blank_black_image,

			nearest_sampler,
			linear_sampler,
			nearest_sampler_repeat,
			linear_sampler_repeat,

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
	#[instrument(skip_all, name="gfx rm handle_resize")]
	pub fn handle_resize(&mut self, core: &mut core::Core) {
		if let Some(_size) = self.resize_request.take() {
			// TODO(pat.m): recreate framebuffers etc
			for image in self.images.iter_mut() {
				image.on_resize(core);
			}

			self.framebuffer_cache.refresh_attachments(core, &self.images);
		}
	}

	#[instrument(skip_all, name="gfx rm start_frame")]
	pub fn start_frame(&mut self, core: &mut core::Core) {
		self.handle_resize(core);

		// TODO(pat.m): maybe this should happen _after_ request processing.
		// otherwise images have to clear themselves on creation.
		core.push_debug_group("Clear Image Resources");
		for image in self.images.iter_mut() {
			if image.clear_policy == ImageClearPolicy::DefaultAtFrameStart {
				core.clear_image_to_default(image.name);
			}
		}
		core.pop_debug_group();
	}

	/// Attempt to turn requested resources into committed GPU resources.
	#[instrument(skip_all, name="gfx rm process_requests")]
	pub fn process_requests(&mut self, core: &mut core::Core, vfs: &vfs::Vfs) -> anyhow::Result<()> {
		core.push_debug_group("Process Resource Requests");

		let _debug_group_guard = common::defer(|| core.pop_debug_group());

		self.load_shader_requests.process_requests(&mut self.shaders, |def| {
			let label = def.path.display().to_string();

			ShaderResource::from_vfs(core, vfs, def.shader_type, &def.path, &label)
				.with_context(|| format!("Compiling shader '{}'", def.path.display()))
		})?;

		self.compile_shader_requests.process_requests(&mut self.shaders, |def| {
			ShaderResource::from_source(core, def.shader_type, &def.src, &def.label)
				.with_context(|| format!("Compiling shader '{}' from source", def.label))
		})?;

		self.load_image_requests.process_requests(&mut self.images, |def| {
			let label = def.path.display().to_string();
			ImageResource::from_vfs(core, vfs, &def.path, label)
				.with_context(|| format!("Loading image '{}'", def.path.display()))
		})?;

		self.load_image_array_requests.process_requests(&mut self.images, |def| {
			ImageResource::array_from_vfs(core, vfs, &def.paths, def.label.clone())
				.with_context(|| format!("Loading image array '{}'", def.label))
		})?;

		self.create_image_requests.process_requests(&mut self.images, |def| {
			Ok(ImageResource::from_create_request(core, def))
		})?;

		Ok(())
	}
}

/// Execution api
impl ResourceManager {
	#[instrument(skip_all, name="gfx rm resolve_draw_pipeline")]
	pub fn resolve_draw_pipeline(&mut self, core: &mut core::Core,
		vertex_shader: shader::ShaderHandle, fragment_shader: impl Into<Option<shader::ShaderHandle>>)
		-> core::ShaderPipelineName
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

	#[instrument(skip_all, name="gfx rm resolve_compute_pipeline")]
	pub fn resolve_compute_pipeline(&mut self, core: &mut core::Core, compute_shader: shader::ShaderHandle)
		-> core::ShaderPipelineName
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

	#[instrument(skip_all, name="gfx rm resolve_framebuffer")]
	pub fn resolve_framebuffer(&mut self, core: &core::Core, desc: impl Into<FramebufferDescription>)
		-> Option<core::FramebufferName>
	{
		self.framebuffer_cache.resolve(core, &self.images, desc.into())
	}

	pub fn get_blank_image(&self, image: BlankImage) -> ImageName {
		match image {
			BlankImage::White => self.blank_white_image,
			BlankImage::Black => self.blank_black_image,
		}
	}

	pub fn get_common_sampler(&self, sampler: CommonSampler) -> SamplerName {
		match sampler {
			CommonSampler::Linear => self.linear_sampler,
			CommonSampler::Nearest => self.nearest_sampler,
			CommonSampler::LinearRepeat => self.linear_sampler_repeat,
			CommonSampler::NearestRepeat => self.nearest_sampler_repeat,
		}
	}

	pub fn get_common_shader(&self, shader: CommonShader) -> ShaderHandle {
		match shader {
			CommonShader::StandardVertex => self.standard_vs_shader,
			CommonShader::FullscreenVertex => self.fullscreen_vs_shader,

			CommonShader::FlatTexturedFragment => self.flat_textured_fs_shader,
		}
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


