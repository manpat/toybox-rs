#![feature(let_chains)]

use toybox_host as host;
use anyhow::Context;

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
	pub use crate::core::ResourceName;
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
		
		unsafe {
			core.gl.Enable(gl::PROGRAM_POINT_SIZE);
			core.gl.Enable(gl::DEPTH_TEST);
		}

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
			.context("Error while processing resource requests")
			.unwrap();

		let clear_color = self.frame_encoder.backbuffer_clear_color;
		let clear_depth = 1.0; // 1.0 is the default clear depth for opengl
		let clear_stencil = 0;

		let backbuffer_handle = core::FboName::backbuffer();
		self.core.clear_framebuffer_color_buffer(backbuffer_handle, 0, clear_color);
		self.core.clear_framebuffer_depth_stencil(backbuffer_handle, clear_depth, clear_stencil);

		// Resolve alignment for staged uploads
		for command_group in self.frame_encoder.command_groups.iter() {
			for command in command_group.commands.iter() {
				let Some(bindings) = command.bindings() else { continue };
				// bindings.resolve_named_bindings();
				bindings.imbue_staged_buffer_alignments(&mut self.frame_encoder.upload_stage, self.core.capabilities());
			}
		}

		// Upload everything
		self.frame_encoder.upload_stage.push_to_heap(&mut self.core, &mut self.resource_manager.upload_heap);

		// Resolve all staged bind sources to concrete names and ranges
		for command_group in self.frame_encoder.command_groups.iter_mut() {
			for command in command_group.commands.iter_mut() {
				let Some(bindings) = command.bindings_mut() else { continue };
				bindings.resolve_staged_bindings(&self.resource_manager.upload_heap);
			}
		}


		// Dispatch commands to GPU
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

        self.resource_manager.upload_heap.create_end_frame_fence(&mut self.core);

		self.core.swap();
		self.frame_encoder.end_frame(&mut self.core);

        self.resource_manager.upload_heap.reset();
	}
}



fn execute_command(command: command::Command, core: &mut Core, resource_manager: &mut ResourceManager) {
	use command::{Command::*, draw};

	match command {
		DebugMessage { label } => {
			core.debug_marker(&label);
		}

		Callback(callback) => {
			callback(core, resource_manager);
		}

		Draw(cmd) => cmd.execute(core, resource_manager),

		_ => unimplemented!(),
	}
}