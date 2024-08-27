use crate::bindings::{self, BindingDescription};
use crate::upload_heap::{UploadStage, UploadHeap};

use crate::{
	Capabilities,
	BufferArgument,
};

pub mod compute;
pub mod draw;

pub use compute::{ComputeCmd, DispatchSize};
pub use draw::{DrawCmd, PrimitiveType};


pub enum Command {
	Draw(DrawCmd),
	Compute(ComputeCmd),

	ClearBuffer,
	ClearTexture,

	CopyBuffer,
	CopyTexture,

	DebugMessage { label: String, },
	PushDebugGroup { label: String, },
	PopDebugGroup,

	Callback(Box<dyn FnOnce(&mut crate::Core, &mut crate::ResourceManager) + 'static>),
}


impl Command {
	pub fn bindings_mut(&mut self) -> Option<&mut BindingDescription> {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, .. }) => Some(bindings),
			Compute(ComputeCmd { bindings, .. }) => Some(bindings),
			_ => None
		}
	}
	
	pub fn bindings(&self) -> Option<&BindingDescription> {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, .. }) => Some(bindings),
			Compute(ComputeCmd { bindings, .. }) => Some(bindings),
			_ => None
		}
	}

	pub fn resolve_staged_buffer_alignments(&self, upload_stage: &mut UploadStage, capabilities: &Capabilities) {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, index_buffer, .. }) => {
				bindings.imbue_staged_buffer_alignments(upload_stage, capabilities);

				if let Some(BufferArgument::Staged(upload_id)) = index_buffer {
					// TODO(pat.m): allow non-32b indices
					upload_stage.update_staged_upload_alignment(*upload_id, 4);
				}
			},

			Compute(ComputeCmd { bindings, dispatch_size, .. }) => {
				bindings.imbue_staged_buffer_alignments(upload_stage, capabilities);

				if let DispatchSize::Indirect(BufferArgument::Staged(upload_id)) = dispatch_size {
					upload_stage.update_staged_upload_alignment(*upload_id, 4);
				}
			},

			_ => {}
		}
	}

	pub fn resolve_staged_bind_sources(&mut self, upload_heap: &mut UploadHeap) {
		use Command::*;

		match self {
			Draw(DrawCmd { bindings, index_buffer, .. }) => {
				bindings.resolve_staged_bind_sources(upload_heap);

				if let Some(bind_source) = index_buffer {
					bindings::resolve_staged_bind_source(bind_source, upload_heap);
				}
			},

			Compute(ComputeCmd { bindings, dispatch_size, .. }) => {
				bindings.resolve_staged_bind_sources(upload_heap);

				if let DispatchSize::Indirect(bind_source) = dispatch_size {
					bindings::resolve_staged_bind_source(bind_source, upload_heap);
				}
			},

			_ => {}
		}
	}
}
