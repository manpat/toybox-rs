use crate::prelude::*;
use crate::{Core, ImageHandle, FramebufferName, ResourceStorage, ImageResource, ImageFormat, FramebufferAttachment};

use std::collections::HashMap;



const MAX_ATTACHMENTS: usize = 5;

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone)]
pub struct FramebufferDescription {
	pub attachments: [Option<ImageHandle>; MAX_ATTACHMENTS],
}

impl FramebufferDescription {
	pub fn is_default(&self) -> bool {
		self.attachments.iter().all(Option::is_none)
	}
}



pub struct FramebufferCache {
	entries: HashMap<FramebufferDescription, Entry>,
}

impl FramebufferCache {
	pub fn new() -> FramebufferCache {
		FramebufferCache {
			entries: HashMap::new(),
		}
	}

	pub fn resolve(&mut self, core: &Core, images: &ResourceStorage<ImageResource>, desc: FramebufferDescription) -> Option<FramebufferName> {
		if desc.is_default() {
			return None
		}

		let name = self.entries.entry(desc)
			.or_insert_with_key(|key| create_entry(core, images, key))
			.name;

		Some(name)
	}

	pub fn refresh_attachments(&mut self, core: &mut Core, images: &ResourceStorage<ImageResource>) {
		for (desc, Entry{ name }) in self.entries.iter() {
			attach_attachments(*name, core, images, desc);
		}
	}
}





struct Entry {
	name: FramebufferName,
	// TODO(pat.m): garbage collect fbos not resolved in a while
	// age: u32,
}



fn create_entry(core: &Core, images: &ResourceStorage<ImageResource>, desc: &FramebufferDescription) -> Entry {
	let name = core.create_framebuffer();
	attach_attachments(name, core, images, desc);
	Entry { name }
}

fn attach_attachments(framebuffer_name: FramebufferName, core: &Core,
	images: &ResourceStorage<ImageResource>, desc: &FramebufferDescription)
{
	let mut color_attachment_idx = 0;
	let mut debug_label = String::from("fbo:");

	for attachment in desc.attachments.iter() {
		let Some(image_handle) = attachment else { continue };
		let Some(image) = images.get_resource(*image_handle) else {
			// TODO(pat.m): doesn't need to panic here
			panic!("Trying to attach invalid handle to framebuffer");
			// continue
		};

		let attachment = match image.image_info.format {
			ImageFormat::Depth | ImageFormat::Depth16 | ImageFormat::Depth32 => FramebufferAttachment::Depth,
			ImageFormat::Stencil => FramebufferAttachment::Stencil,
			ImageFormat::DepthStencil => FramebufferAttachment::DepthStencil,
			_ => {
				let idx = color_attachment_idx;
				color_attachment_idx += 1;
				FramebufferAttachment::Color(idx)
			}
		};

		core.set_framebuffer_attachment(framebuffer_name, attachment, image.name);

		if !image.label.is_empty() {
			debug_label.push_str(&format!("{attachment:?}:\"{}\" ", &image.label));
		} else {
			debug_label.push_str(&format!("{attachment:?}:#{} ", image.name.as_raw()));
		}
	}

	core.set_debug_label(framebuffer_name, &debug_label);
}



impl From<&[ImageHandle]> for FramebufferDescription {
	fn from(handles: &[ImageHandle]) -> Self {
		assert!(handles.len() < MAX_ATTACHMENTS);

		let mut attachments = [None; MAX_ATTACHMENTS];
		for (attachment, handle) in std::iter::zip(&mut attachments, handles) {
			*attachment = Some(*handle);
		}

		FramebufferDescription { attachments }
	}
}

impl<const N: usize> From<&[ImageHandle; N]> for FramebufferDescription {
	fn from(handles: &[ImageHandle; N]) -> Self {
		assert!(N < MAX_ATTACHMENTS);

		let mut attachments = [None; MAX_ATTACHMENTS];
		for (attachment, handle) in std::iter::zip(&mut attachments, handles) {
			*attachment = Some(*handle);
		}
		FramebufferDescription { attachments }
	}
}
