use crate::prelude::*;
use crate::core::*;
use crate::resource_manager::{ResourceManager, arguments::*};
use crate::upload_heap::{UploadStage, UploadHeap};


// TODO: string interning would be great
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum BufferBindTarget {
	UboIndex(u32),
	SsboIndex(u32),
	Named(&'static str),
}

// TODO: string interning would be great
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum ImageBindTarget {
	Sampled(u32),
	ReadonlyImage(u32),
	ReadWriteImage(u32),
	Named(&'static str),
}

impl BufferBindTarget {
	pub fn to_indexed_buffer_target(&self) -> Option<IndexedBufferTarget> {
		match self {
			Self::UboIndex(_) => Some(IndexedBufferTarget::Uniform),
			Self::SsboIndex(_) => Some(IndexedBufferTarget::ShaderStorage),
			_ => None,
		}
	}

	pub fn to_raw_index(&self) -> Option<u32> {
		match self {
			Self::UboIndex(index) | Self::SsboIndex(index) => Some(*index),
			_ => None,
		}
	}
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct BufferBindDesc {
	pub target: BufferBindTarget,
	pub source: BufferArgument,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ImageBindDesc {
	pub target: ImageBindTarget,
	pub source: ImageArgument,
	pub sampler: Option<SamplerArgument>,
}


#[derive(Debug, Default)]
pub struct BindingDescription {
	// TODO(pat.m): store unresolved named targets separately to resolved/explicit targets to simplify usage 
	pub buffer_bindings: SmallVec<[BufferBindDesc; 4]>,
	pub image_bindings: SmallVec<[ImageBindDesc; 4]>,

	pub framebuffer: Option<FramebufferArgument>,
}


impl BindingDescription {
	pub fn new() -> BindingDescription {
		BindingDescription::default()
	}

	pub fn clear(&mut self) {
		self.buffer_bindings.clear();
		self.image_bindings.clear();
	}

	pub fn bind_buffer(&mut self, target: impl Into<BufferBindTarget>, source: impl Into<BufferArgument>) {
		self.buffer_bindings.push(BufferBindDesc {
			target: target.into(),
			source: source.into(),
		});
	}

	pub fn bind_image(&mut self, target: impl Into<ImageBindTarget>, source: impl Into<ImageArgument>) {
		self.image_bindings.push(ImageBindDesc {
			target: target.into(),
			source: source.into(),
			sampler: None,
		});
	}

	pub fn bind_sampled_image(&mut self, target: impl Into<ImageBindTarget>, source: impl Into<ImageArgument>, sampler: impl Into<SamplerArgument>) {
		self.image_bindings.push(ImageBindDesc {
			target: target.into(),
			source: source.into(),
			sampler: Some(sampler.into()),
		});
	}

	/// Bind for the duration of the binding scope, either a FramebufferName, a framebuffer described by a FramebufferDescription, or
	/// the default framebuffer (None).
	pub fn bind_framebuffer(&mut self, framebuffer: impl Into<FramebufferArgument>) {
		self.framebuffer = Some(framebuffer.into());
	}

	pub fn resolve_named_bind_targets(&mut self) {
		// TODO(pat.m): resolve BufferBindTarget::Named to UboIndex or SsboIndex
		// ImageBindTarget::Named to Unit
		// Needs shader reflection
	}

	pub fn imbue_staged_buffer_alignments(&self, upload_stage: &mut UploadStage, capabilities: &Capabilities) {
		for bind_desc in self.buffer_bindings.iter() {
			let BufferArgument::Staged(upload_id) = bind_desc.source else { continue };

			// https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#subsection.6.7.1
			let alignment = match bind_desc.target {
				BufferBindTarget::UboIndex(_) => capabilities.ubo_bind_alignment,
				BufferBindTarget::SsboIndex(_) => capabilities.ssbo_bind_alignment,
				_ => panic!("Named buffer bind target encountered in imbue_staged_buffer_alignments. Names must be resolved before this point"),
			};

			upload_stage.update_staged_upload_alignment(upload_id, alignment);
		}
	}

	pub fn resolve_staged_bind_sources(&mut self, upload_heap: &UploadHeap) {
		for bind_desc in self.buffer_bindings.iter_mut() {
			resolve_staged_bind_source(&mut bind_desc.source, upload_heap);
		}
	}

	pub fn resolve_image_bind_sources(&mut self, rm: &mut ResourceManager) {
		for ImageBindDesc{source, ..} in self.image_bindings.iter_mut() {
			let name = match *source {
				ImageArgument::Handle(handle) => rm.images.get_name(handle).expect("Failed to resolve image handle"),
				ImageArgument::Blank(image) => rm.get_blank_image(image),
				ImageArgument::Name(_) => continue,
			};

			*source = ImageArgument::Name(name);
		}
	}

	pub fn merge_unspecified_from(&mut self, other: &BindingDescription) {
		let num_initial_buffer_bindings = self.buffer_bindings.len();
		let num_initial_image_bindings = self.image_bindings.len();

		for needle in other.buffer_bindings.iter() {
			let haystack = &self.buffer_bindings[..num_initial_buffer_bindings];
			if haystack.iter().all(|h| h.target != needle.target) {
				self.buffer_bindings.push(needle.clone());
			}
		}

		for needle in other.image_bindings.iter() {
			let haystack = &self.image_bindings[..num_initial_image_bindings];
			if haystack.iter().all(|h| h.target != needle.target) {
				self.image_bindings.push(needle.clone());
			}
		}

		if self.framebuffer.is_none() {
			self.framebuffer = other.framebuffer.clone();
		}
	}

	// TODO(pat.m): not sure if I want to do this here.
	// It does limit things a bit if I want to look things up in a per-pass BindingDescription.
	// Also binding should probably be done through a bindings tracker.
	#[tracing::instrument(skip_all, name="BindingDescription::bind")]
	pub fn bind(&self, core: &mut Core, resource_manager: &mut ResourceManager) {
		let mut barrier_tracker = core.barrier_tracker();

		{
			let _span = tracing::info_span!("bind buffers").entered();

			for BufferBindDesc{target, source} in self.buffer_bindings.iter() {
				let BufferArgument::Name{name, range} = *source
					else { panic!("Unresolved buffer bind source") };

				let Some((index, indexed_target)) = target.to_raw_index().zip(target.to_indexed_buffer_target())
					else { panic!("Unresolved buffer target") };

				match indexed_target {
					// TODO(pat.m): this is pessimistic - but we need shader reflection to guarantee that an ssbo is bound
					// as readonly.
					IndexedBufferTarget::ShaderStorage => barrier_tracker.write_buffer(name, gl::SHADER_STORAGE_BARRIER_BIT),
					IndexedBufferTarget::Uniform => barrier_tracker.read_buffer(name, gl::UNIFORM_BARRIER_BIT),
				}

				core.bind_indexed_buffer(indexed_target, index, name, range);
			}
		}

		{
			let _span = tracing::info_span!("bind images").entered();

			for ImageBindDesc{target, source, sampler} in self.image_bindings.iter() {
				let ImageArgument::Name(image_name) = *source
					else { panic!("Unresolved image bind source") };

				match *target {
					ImageBindTarget::Sampled(unit) => {
						barrier_tracker.read_image(image_name, gl::TEXTURE_FETCH_BARRIER_BIT);

						// TODO(pat.m): use default instead of panicking
						let sampler_name = match sampler.expect("Sampled bind target missing sampler") {
							SamplerArgument::Name(name) => name,
							SamplerArgument::Common(sampler) => resource_manager.get_common_sampler(sampler),
						};

						core.bind_sampler(unit, sampler_name);
						core.bind_sampled_image(unit, image_name);
					}

					ImageBindTarget::ReadonlyImage(unit) => {
						barrier_tracker.read_image(image_name, gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
						core.bind_image(unit, image_name);
					}

					ImageBindTarget::ReadWriteImage(unit) => {
						barrier_tracker.write_image(image_name, gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
						core.bind_image_rw(unit, image_name);
					}

					_ => panic!("Unresolved image bind target"),
				}
			}
		}


		{
			let _span = tracing::info_span!("bind framebuffer").entered();

			// TODO(pat.m): the following should only be done for the Draw command really.
			// it doesn't make sense to bind or emit barriers for e.g., Compute.

			// Framebuffer should _ALWAYS_ be defined by this point.
			// The global BindingDescription should specify Default
			let framebuffer = self.framebuffer.as_ref()
				.expect("Unresolved framebuffer")
				.resolve_name(core, resource_manager);

			if let Some(framebuffer_name) = framebuffer {
				let framebuffer_info = core.get_framebuffer_info(framebuffer_name);
				for attachment_image in framebuffer_info.attachments.values() {
					// NOTE: only a read barrier since framebuffer writes are implicitly synchronised with later draw calls.
					// We only need to make sure that if an image is _modified_ that a barrier is inserted before rendering to it.
					barrier_tracker.read_image(*attachment_image, gl::FRAMEBUFFER_BARRIER_BIT);
				}

				let framebuffer_size = core.get_framebuffer_size(framebuffer_name);
				core.set_viewport(framebuffer_size);
			} else {
				core.set_viewport(core.backbuffer_size());
			}

			core.bind_framebuffer(framebuffer);
		}
	}
}

pub fn resolve_staged_bind_source(source: &mut BufferArgument, upload_heap: &UploadHeap) {
	if let BufferArgument::Staged(upload_id) = *source {
		let allocation = upload_heap.resolve_allocation(upload_id);
		*source = BufferArgument::Name {
			name: upload_heap.buffer_name(),
			range: Some(allocation),
		};
	}
}

