use crate::prelude::*;
use crate::core::*;
use crate::resource_manager::{ResourceManager, ImageHandle, FramebufferDescription};
use crate::upload_heap::{UploadStage, UploadHeap, StagedUploadId};


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
pub enum BufferBindSource {
	Name {
		name: BufferName, 
		range: Option<BufferRange>,
	},
	Staged(StagedUploadId),
}

impl From<StagedUploadId> for BufferBindSource {
	fn from(upload_id: StagedUploadId) -> Self {
		Self::Staged(upload_id)
	}
}

impl From<BufferName> for BufferBindSource {
	fn from(name: BufferName) -> Self {
		Self::Name{name, range: None}
	}
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum ImageBindSource {
	Name(ImageName),
	Handle(ImageHandle),
}

impl From<ImageName> for ImageBindSource {
	fn from(name: ImageName) -> Self {
		Self::Name(name)
	}
}

impl From<ImageHandle> for ImageBindSource {
	fn from(handle: ImageHandle) -> Self {
		Self::Handle(handle)
	}
}


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct BufferBindDesc {
	pub target: BufferBindTarget,
	pub source: BufferBindSource,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ImageBindDesc {
	pub target: ImageBindTarget,
	pub source: ImageBindSource,
	pub sampler: Option<SamplerName>,
}


#[derive(Debug)]
pub enum FramebufferDescriptionOrName {
	Default,
	Name(FramebufferName),
	Description(FramebufferDescription),
}


#[derive(Debug, Default)]
pub struct BindingDescription {
	// TODO(pat.m): store unresolved named targets separately to resolved/explicit targets to simplify usage 
	pub buffer_bindings: Vec<BufferBindDesc>,
	pub image_bindings: Vec<ImageBindDesc>,

	pub framebuffer: Option<FramebufferDescriptionOrName>,
}


impl BindingDescription {
	pub fn new() -> BindingDescription {
		BindingDescription::default()
	}

	pub fn clear(&mut self) {
		self.buffer_bindings.clear();
		self.image_bindings.clear();
	}

	pub fn bind_buffer(&mut self, target: impl Into<BufferBindTarget>, source: impl Into<BufferBindSource>) {
		self.buffer_bindings.push(BufferBindDesc {
			target: target.into(),
			source: source.into(),
		});
	}

	pub fn bind_image(&mut self, target: impl Into<ImageBindTarget>, source: impl Into<ImageBindSource>, sampler: impl Into<Option<SamplerName>>) {
		self.image_bindings.push(ImageBindDesc {
			target: target.into(),
			source: source.into(),
			sampler: sampler.into(),
		});
	}

	/// Bind for the duration of the binding scope, either a FramebufferName, a framebuffer described by a FramebufferDescription, or
	/// the default framebuffer (None).
	pub fn bind_framebuffer(&mut self, framebuffer: impl Into<FramebufferDescriptionOrName>) {
		self.framebuffer = Some(framebuffer.into());
	}

	pub fn resolve_named_bind_targets(&mut self) {
		// TODO(pat.m): resolve BufferBindTarget::Named to UboIndex or SsboIndex
		// ImageBindTarget::Named to Unit
		// Needs shader reflection
	}

	pub fn imbue_staged_buffer_alignments(&self, upload_stage: &mut UploadStage, capabilities: &Capabilities) {
		for bind_desc in self.buffer_bindings.iter() {
			let BufferBindSource::Staged(upload_id) = bind_desc.source else { continue };

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
			if let ImageBindSource::Handle(handle) = *source {
				let name = rm.images.get_name(handle)
					.expect("Failed to resolve image handle");

				*source = ImageBindSource::Name(name);
			}
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
	}

	// TODO(pat.m): not sure if I want to do this here.
	// It does limit things a bit if I want to look things up in a per-pass BindingDescription.
	// Also binding should probably be done through a bindings tracker.
	pub fn bind(&self, core: &mut Core, resource_manager: &mut ResourceManager) {
		let mut barrier_tracker = core.barrier_tracker();

		for BufferBindDesc{target, source} in self.buffer_bindings.iter() {
			let BufferBindSource::Name{name, range} = *source
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

		for ImageBindDesc{target, source, sampler} in self.image_bindings.iter() {
			let ImageBindSource::Name(image_name) = *source
				else { panic!("Unresolved image bind source") };

			match *target {
				ImageBindTarget::Sampled(unit) => { 
					barrier_tracker.read_image(image_name, gl::TEXTURE_FETCH_BARRIER_BIT);

					let sampler_name = sampler.expect("Sampled bind target missing sampler");
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

		// TODO(pat.m): framebuffer should _ALWAYS_ be defined by this point.
		// The global BindingDescription should specify Default
		let framebuffer = self.framebuffer.as_ref().unwrap_or(&FramebufferDescriptionOrName::Default)
			.resolve_name(core, resource_manager);

		core.bind_framebuffer(framebuffer);
	}
}

pub fn resolve_staged_bind_source(source: &mut BufferBindSource, upload_heap: &UploadHeap) {
	if let BufferBindSource::Staged(upload_id) = *source {
		let allocation = upload_heap.resolve_allocation(upload_id);
		*source = BufferBindSource::Name {
			name: upload_heap.buffer_name(),
			range: Some(allocation),
		};
	}
}



pub trait IntoBufferBindSourceOrStageable {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSource;
}

impl IntoBufferBindSourceOrStageable for crate::upload_heap::StagedUploadId {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSource {
		self.into()
	}
}

impl IntoBufferBindSourceOrStageable for crate::core::BufferName {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSource {
		self.into()
	}
}

// Accept anything that can be turned into a slice of sized, copyable items - including regular references
impl<'t, T> IntoBufferBindSourceOrStageable for &'t T
	where T: crate::AsStageableSlice
{
	fn into_bind_source(self, stage: &mut UploadStage) -> BufferBindSource {
		stage.stage_data(self.as_slice()).into()
	}
}




impl<T> From<T> for FramebufferDescriptionOrName
	where T: Into<FramebufferDescription>
{
	fn from(o: T) -> Self {
		FramebufferDescriptionOrName::Description(o.into())
	}
}


impl From<FramebufferName> for FramebufferDescriptionOrName {
	fn from(o: FramebufferName) -> Self {
		FramebufferDescriptionOrName::Name(o)
	}
}

impl FramebufferDescriptionOrName {
	pub fn resolve_name(&self, core: &Core, resource_manager: &mut ResourceManager) -> Option<FramebufferName> {
		match self {
			FramebufferDescriptionOrName::Default => None,
			FramebufferDescriptionOrName::Name(name) => Some(*name),
			FramebufferDescriptionOrName::Description(desc) => resource_manager.resolve_framebuffer(core, desc.clone()),
		}
	}
}