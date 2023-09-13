use crate::prelude::*;
use crate::core::{self, Core, BufferName, Capabilities};
use crate::core::buffer::{IndexedBufferTarget, BufferRange};
use crate::upload_heap::{UploadStage, UploadHeap, StagedUploadId};


// TODO: string interning would be great
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum BufferBindTargetDesc {
	UboIndex(u32),
	SsboIndex(u32),
	Named(&'static str),
}

impl BufferBindTargetDesc {
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
pub enum BufferBindSourceDesc {
	Name {
		name: BufferName, 
		range: Option<BufferRange>,
	},
	Staged(StagedUploadId),
}

impl From<StagedUploadId> for BufferBindSourceDesc {
	fn from(upload_id: StagedUploadId) -> Self {
		Self::Staged(upload_id)
	}
}

impl From<BufferName> for BufferBindSourceDesc {
	fn from(name: BufferName) -> Self {
		Self::Name{name, range: None}
	}
}



#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct BufferBindDesc {
	pub target: BufferBindTargetDesc,
	pub source: BufferBindSourceDesc,
}


#[derive(Debug, Default)]
pub struct BindingDescription {
	pub buffer_bindings: Vec<BufferBindDesc>,
	// Image bindings
}


impl BindingDescription {
	pub fn new() -> BindingDescription {
		BindingDescription::default()
	}

	pub fn clear(&mut self) {
		self.buffer_bindings.clear();
	}

	pub fn bind_buffer(&mut self, target: impl Into<BufferBindTargetDesc>, source: impl Into<BufferBindSourceDesc>) {
		self.buffer_bindings.push(BufferBindDesc {
			target: target.into(),
			source: source.into(),
		});
	}

	pub fn resolve_named_bind_targets(&mut self) {
		// TODO(pat.m): resolve BufferBindTargetDesc::Named to UboIndex or SsboIndex
		// Needs shader reflection
	}

	pub fn imbue_staged_buffer_alignments(&self, upload_stage: &mut UploadStage, capabilities: &Capabilities) {
		for bind_desc in self.buffer_bindings.iter() {
			let BufferBindSourceDesc::Staged(upload_id) = bind_desc.source else { continue };

			// https://registry.khronos.org/OpenGL/specs/gl/glspec45.core.pdf#subsection.6.7.1
			let alignment = match bind_desc.target {
				BufferBindTargetDesc::UboIndex(_) => capabilities.ubo_bind_alignment,
				BufferBindTargetDesc::SsboIndex(_) => capabilities.ssbo_bind_alignment,
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

	pub fn merge_unspecified_from(&mut self, other: &BindingDescription) {
		let num_initial_bindings = self.buffer_bindings.len();

		for needle in other.buffer_bindings.iter() {
			let haystack = &self.buffer_bindings[..num_initial_bindings];
			if haystack.iter().all(|h| h.target != needle.target) {
				self.buffer_bindings.push(needle.clone());
			}
		}
	}

	// TODO(pat.m): not sure if I want to do this here.
	// It does limit things a bit if I want to look things up in a per-pass BindingDescription.
	// Also binding should probably be done through a bindings tracker.
	pub fn bind(&self, core: &mut Core) {
		let mut barrier_tracker = core.barrier_tracker();

		for BufferBindDesc{target, source} in self.buffer_bindings.iter() {
			let BufferBindSourceDesc::Name{name, range} = *source
				else { panic!("Unresolved buffer bind source description") };

			let Some((index, indexed_target)) = target.to_raw_index().zip(target.to_indexed_buffer_target())
				else { panic!("Unresolve buffer target description") };

			match indexed_target {
				// TODO(pat.m): this is pessimistic - but we need shader reflection to guarantee that an ssbo is bound
				// as readonly.
				IndexedBufferTarget::ShaderStorage => barrier_tracker.write_buffer(name, gl::SHADER_STORAGE_BARRIER_BIT),
				IndexedBufferTarget::Uniform => barrier_tracker.read_buffer(name, gl::UNIFORM_BARRIER_BIT),
			}

			core.bind_indexed_buffer(indexed_target, index, name, range);
		}
	}
}

pub fn resolve_staged_bind_source(source: &mut BufferBindSourceDesc, upload_heap: &UploadHeap) {
	if let BufferBindSourceDesc::Staged(upload_id) = *source {
		let allocation = upload_heap.resolve_allocation(upload_id);
		*source = BufferBindSourceDesc::Name {
			name: upload_heap.buffer_name(),
			range: Some(allocation),
		};
	}
}



pub trait IntoBufferBindSourceOrStageable {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSourceDesc;
}

impl IntoBufferBindSourceOrStageable for crate::upload_heap::StagedUploadId {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSourceDesc {
		self.into()
	}
}

impl IntoBufferBindSourceOrStageable for crate::core::BufferName {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferBindSourceDesc {
		self.into()
	}
}

// Accept anything that can be turned into a slice of sized, copyable items - including regular references
impl<'t, T, U> IntoBufferBindSourceOrStageable for &'t T
	where T: crate::AsSlice<Target=U>
		, U: Copy + Sized + 'static
{
	fn into_bind_source(self, stage: &mut UploadStage) -> BufferBindSourceDesc {
		stage.stage_data(self.as_slice()).into()
	}
}