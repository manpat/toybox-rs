use crate::{
	ImageName,
	ImageHandle,
};


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum ImageArgument {
	Name(ImageName),
	Handle(ImageHandle),
	Blank(BlankImage),
}

impl From<ImageName> for ImageArgument {
	fn from(name: ImageName) -> Self {
		Self::Name(name)
	}
}

impl From<ImageHandle> for ImageArgument {
	fn from(handle: ImageHandle) -> Self {
		Self::Handle(handle)
	}
}

impl From<BlankImage> for ImageArgument {
	fn from(handle: BlankImage) -> Self {
		Self::Blank(handle)
	}
}



#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum BlankImage {
	White,
	Black,
}