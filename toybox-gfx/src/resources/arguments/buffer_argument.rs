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

impl From<(BufferName, BufferRange)> for BufferArgument {
	fn from((name, range): (BufferName, BufferRange)) -> Self {
		Self::Name{name, range: Some(range)}
	}
}




pub trait IntoBufferArgument {
	fn into_buffer_argument(self, _: &mut UploadStage) -> BufferArgument;
}

impl IntoBufferArgument for StagedUploadId {
	fn into_buffer_argument(self, _: &mut UploadStage) -> BufferArgument {
		self.into()
	}
}

impl IntoBufferArgument for BufferName {
	fn into_buffer_argument(self, _: &mut UploadStage) -> BufferArgument {
		self.into()
	}
}

impl IntoBufferArgument for (BufferName, BufferRange) {
	fn into_buffer_argument(self, _: &mut UploadStage) -> BufferArgument {
		self.into()
	}
}

impl IntoBufferArgument for BufferArgument {
	fn into_buffer_argument(self, _: &mut UploadStage) -> BufferArgument {
		self
	}
}


// Accept anything that can be turned into a slice of sized, copyable items - including regular references
impl<'t, T> IntoBufferArgument for &'t T
	where T: AsStageableSlice
{
	fn into_buffer_argument(self, stage: &mut UploadStage) -> BufferArgument {
		stage.stage_data(self.as_slice()).into()
	}
}



pub trait BufferRangeExt {
	fn with_offset_size(&self, offset: u32, size: u32) -> BufferArgument;
}

impl BufferRangeExt for BufferName {
	fn with_offset_size(&self, offset: u32, size: u32) -> BufferArgument {
		let range = BufferRange {
			offset: offset as usize,
			size: size as usize,
		};

		BufferArgument::Name {
			name: *self,
			range: Some(range),
		}
	}
}