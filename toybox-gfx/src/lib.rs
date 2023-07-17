use toybox_host as host;

use host::prelude::*;
use host::gl;

pub mod core;
pub mod bindings;
pub mod command;
pub mod command_group;
pub mod frame_encoder;
pub mod resource_manager;
pub mod upload_heap;

pub use crate::core::Core;
pub use resource_manager::ResourceManager;
pub use frame_encoder::FrameEncoder;

pub mod prelude {
	pub use crate::host::gl;
}


pub struct System {
	pub core: core::Core,
	pub resource_manager: resource_manager::ResourceManager,
	pub frame_encoder: frame_encoder::FrameEncoder,
}


impl System {
	pub fn new(mut core: core::Core) -> anyhow::Result<System> {
		let resource_manager = resource_manager::ResourceManager::new(&mut core);
		let frame_encoder = frame_encoder::FrameEncoder::new(&mut core);
		
		Ok(System {
			core,
			resource_manager,
			frame_encoder,
		})
	}

	pub fn resize(&mut self, new_size: common::Vec2i) {
		self.resource_manager.request_resize(new_size);
	}

	pub fn execute_frame(&mut self) {
		self.resource_manager.process_requests(&mut self.core)
			.expect("Failed to process resource requests");

		let clear_color = self.frame_encoder.backbuffer_clear_color;
		let clear_depth = 1.0; // 1.0 is the default clear depth for opengl
		let clear_stencil = 0;

		let backbuffer_handle = core::FboHandle::backbuffer();
		self.core.clear_framebuffer_color_buffer(backbuffer_handle, 0, clear_color);
		self.core.clear_framebuffer_depth_stencil(backbuffer_handle, clear_depth, clear_stencil);

		for command_group in self.frame_encoder.command_groups.iter_mut() {
			if command_group.commands.is_empty() {
				continue
			}

			self.core.push_debug_group(&command_group.label);

			for command in command_group.commands.drain(..) {
				execute_command(command, &mut self.core, &mut self.resource_manager);
			}

			self.core.pop_debug_group();
		}

		self.core.swap();
		self.frame_encoder.reset();
	}
}



fn execute_command(command: command::Command, core: &mut Core, resource_manager: &mut ResourceManager) {
	use command::{Command::*};

	match command {
		DebugMessage { label } => {
			core.debug_marker(&label);
		}

		Callback(callback) => {
			callback(core, resource_manager);
		}

		_ => unimplemented!(),
	}
}