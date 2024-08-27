use crate::{
	AsStageableSlice,

	BufferName,
	BufferRange,

	upload_heap::{
		UploadStage,
		StagedUploadId,
	},
};


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum BufferArgument {
	Name {
		name: BufferName,
		range: Option<BufferRange>,
	},
	Staged(StagedUploadId),
}

impl From<StagedUploadId> for BufferArgument {
	fn from(upload_id: StagedUploadId) -> Self {
		Self::Staged(upload_id)
	}
}

impl From<BufferName> for BufferArgument {
	fn from(name: BufferName) -> Self {
		Self::Name{name, range: None}
	}
}




pub trait IntoBufferArgument {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferArgument;
}

impl IntoBufferArgument for StagedUploadId {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferArgument {
		self.into()
	}
}

impl IntoBufferArgument for crate::core::BufferName {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferArgument {
		self.into()
	}
}

impl IntoBufferArgument for BufferArgument {
	fn into_bind_source(self, _: &mut UploadStage) -> BufferArgument {
		self
	}
}

// Accept anything that can be turned into a slice of sized, copyable items - including regular references
impl<'t, T> IntoBufferArgument for &'t T
	where T: AsStageableSlice
{
	fn into_bind_source(self, stage: &mut UploadStage) -> BufferArgument {
		stage.stage_data(self.as_slice()).into()
	}
}