#![feature(let_chains)]

use toybox_host as host;
use anyhow::Context;

pub mod bindings;
pub mod command;
pub mod command_group;
pub mod core;
pub mod frame_encoder;
pub mod resource_manager;
pub mod shaders;
pub mod upload_heap;

pub use crate::core::*;
pub use resource_manager::*;
pub use frame_encoder::*;
pub use command::PrimitiveType;
pub use command_group::*;
pub use shaders::*;

pub mod prelude {
	pub use crate::host::gl;
	pub use crate::core::ResourceName;

	pub use common::math::*;
}

pub use prelude::*;


pub struct System {
	pub core: core::Core,
	pub resource_manager: resource_manager::ResourceManager,
	pub frame_encoder: frame_encoder::FrameEncoder,
}

impl System {
	pub fn backbuffer_size(&self) -> Vec2i {
		self.core.backbuffer_size()
	}

	pub fn backbuffer_aspect(&self) -> f32 {
		self.core.backbuffer_size().x as f32 / self.core.backbuffer_size().y as f32
	}
}

impl System {
	pub fn new(mut core: core::Core) -> anyhow::Result<System> {
		core.register_debug_hook();

		let resource_manager = resource_manager::ResourceManager::new(&mut core)?;
		let frame_encoder = frame_encoder::FrameEncoder::new(&mut core);
		
		unsafe {
			core.gl.Enable(gl::PROGRAM_POINT_SIZE);
			core.gl.Enable(gl::DEPTH_TEST);

			// Make sure sRGB handling is enabled by default.
			core.gl.Enable(gl::FRAMEBUFFER_SRGB);
		}

		Ok(System {
			core,
			resource_manager,
			frame_encoder,
		})
	}

	pub fn resize(&mut self, new_size: common::Vec2i) {
		if self.core.backbuffer_size() != new_size {
			self.core.set_backbuffer_size(new_size);
			self.resource_manager.request_resize(new_size);
		}
	}

	pub fn start_frame(&mut self) {
		self.core.set_debugging_enabled(true);

		self.resource_manager.start_frame(&mut self.core);
		self.frame_encoder.start_frame();
	}

	pub fn execute_frame(&mut self, vfs: &toybox_vfs::Vfs) {
		self.resource_manager.process_requests(&mut self.core, vfs)
			.context("Error while processing resource requests")
			.unwrap();

		self.frame_encoder.command_groups.sort_by_key(|cg| cg.stage);

		let clear_color = self.frame_encoder.backbuffer_clear_color;
		let clear_depth = 1.0; // 1.0 is the default clear depth for opengl
		let clear_stencil = 0;

		let backbuffer_handle = FramebufferName::backbuffer();
		self.core.clear_framebuffer_color_buffer(backbuffer_handle, 0, clear_color);
		self.core.clear_framebuffer_depth_stencil(backbuffer_handle, clear_depth, clear_stencil);

		let backbuffer_size = self.core.backbuffer_size();
		self.core.set_viewport(backbuffer_size);

		// TODO(pat.m): doing this first may mean duplicate bindings per bind target after name resolution.
		// moving from binding merging to a hierarchical lookup, or to a just-in-time lookup might improve this
		self.merge_bindings();

		self.resolve_named_bind_targets();
		self.resolve_image_bind_sources();

		// Resolve alignment for staged uploads
		self.resolve_staged_buffer_alignments();

		// Upload everything
		self.frame_encoder.upload_stage.push_to_heap(&mut self.core, &mut self.resource_manager.upload_heap);

		// Resolve all staged bind sources to concrete names and ranges
		self.resolve_staged_bind_sources();

		// Dispatch commands to GPU
		self.dispatch_commands();

        self.resource_manager.upload_heap.create_end_frame_fence(&mut self.core);

		self.frame_encoder.end_frame();
        self.resource_manager.upload_heap.reset();

		// HACK: For some reason capturing the app with discord (and I suspect other window capture apps)
		// causes swap_buffers to emit GL_INVALID_ENUM on my machine, which panics ofc.
		// So for now, we are just not emitting errors pls.
		self.core.set_debugging_enabled(false);
	}

	fn resolve_named_bind_targets(&mut self) {
		for command_group in self.frame_encoder.command_groups.iter_mut() {
			for command in command_group.commands.iter_mut() {
				if let Some(bindings) = command.bindings_mut() {
					bindings.resolve_named_bind_targets(/*shaders, resource manager*/);
				}
			}
		}
	}

	fn resolve_staged_buffer_alignments(&mut self) {
		let upload_stage = &mut self.frame_encoder.upload_stage;
		let capabilities = self.core.capabilities();

		// self.frame_encoder.global_bindings.imbue_staged_buffer_alignments(upload_stage, capabilities);

		for command_group in self.frame_encoder.command_groups.iter() {
			// command_group.shared_bindings.imbue_staged_buffer_alignments(upload_stage, capabilities);

			for command in command_group.commands.iter() {
				command.resolve_staged_buffer_alignments(upload_stage, capabilities);
			}
		}
	}

	fn resolve_staged_bind_sources(&mut self) {
		let upload_heap = &mut self.resource_manager.upload_heap;

		// self.frame_encoder.global_bindings.resolve_staged_bind_sources(upload_heap);

		for command_group in self.frame_encoder.command_groups.iter_mut() {
			// command_group.shared_bindings.resolve_staged_bind_sources(upload_heap);

			for command in command_group.commands.iter_mut() {
				command.resolve_staged_bind_sources(upload_heap);
			}
		}
	}

	fn resolve_image_bind_sources(&mut self) {
		for command_group in self.frame_encoder.command_groups.iter_mut() {
			for command in command_group.commands.iter_mut() {
				if let Some(bindings) = command.bindings_mut() {
					bindings.resolve_image_bind_sources(&mut self.resource_manager);
				}
			}
		}
	}

	// TODO(pat.m): this sucks. it would be better for commands to 'pull' the bindings they need
	// rather than bindings be 'pushed' like this - although a binding tracker may make this less bad.
	// its still pretty wasteful though.
	fn merge_bindings(&mut self) {
		for command_group in self.frame_encoder.command_groups.iter_mut() {
			command_group.shared_bindings.merge_unspecified_from(&self.frame_encoder.global_bindings);

			for command in command_group.commands.iter_mut() {
				if let Some(bindings) = command.bindings_mut() {
					bindings.merge_unspecified_from(&command_group.shared_bindings);
				}
			}
		}
	}

	fn dispatch_commands(&mut self) {
		use command::Command::*;

		let core = &mut self.core;
		let resource_manager = &mut self.resource_manager;

		for command_group in self.frame_encoder.command_groups.iter_mut() {
			if command_group.commands.is_empty() {
				continue
			}

			core.push_debug_group(&format!("{:?}", command_group.stage));

			for command in command_group.commands.drain(..) {
				match command {
					DebugMessage { label } => {
						core.debug_marker(&label);
					}

					PushDebugGroup { label } => {
						core.push_debug_group(&label);
					}

					PopDebugGroup => {
						core.pop_debug_group();
					}

					Callback(callback) => callback(core, resource_manager),

					Draw(cmd) => cmd.execute(core, resource_manager),
					Compute(cmd) => cmd.execute(core, resource_manager),

					_ => unimplemented!(),
				}
			}

			core.pop_debug_group();
		}
	}
}



pub trait AsStageableSlice {
	type Target : Copy + Sized + 'static;
	fn as_slice(&self) -> &[Self::Target];
}

impl<T> AsStageableSlice for [T]
	where T: Copy + Sized + 'static
{
	type Target = T;
	fn as_slice(&self) -> &[T] {
		self
	}
}

impl<T, const N: usize> AsStageableSlice for [T; N]
	where T: Copy + Sized + 'static
{
	type Target = T;
	fn as_slice(&self) -> &[T] {
		self
	}
}

impl<T> AsStageableSlice for Vec<T>
	where T: Copy + Sized + 'static
{
	type Target = T;
	fn as_slice(&self) -> &[T] {
		self
	}
}


// TODO(pat.m): move somewhere else
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Axis {
	X, Y, Z
}