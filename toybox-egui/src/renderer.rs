use toybox_gfx as gfx;
use crate::prelude::*;
use gfx::prelude::*;

use egui::{TextureId, ClippedPrimitive};
use epaint::Primitive;
use epaint::image::ImageDelta;

use gfx::core::*;
use gfx::resource_manager::*;


const VERTEX_SOURCE: &str = include_str!("egui.vs.glsl");
const FRAGMENT_SOURCE: &str = include_str!("egui.fs.glsl");


pub struct Renderer {
	sampler: SamplerName,

	vertex_shader: ShaderHandle,
	fragment_shader: ShaderHandle,
}

impl Renderer {
	pub fn new(gfx: &mut gfx::System) -> anyhow::Result<Renderer> {
		let sampler = gfx.core.create_sampler();
		gfx.core.set_sampler_minify_filter(sampler, FilterMode::Nearest, None);
		gfx.core.set_sampler_magnify_filter(sampler, FilterMode::Nearest);
		gfx.core.set_sampler_addressing_mode(sampler, AddressingMode::Clamp);
		gfx.core.set_debug_label(sampler, "egui sampler");

		Ok(Renderer {
			sampler,

			vertex_shader: gfx.resource_manager.compile_shader(CompileShaderRequest::vertex("egui vs", VERTEX_SOURCE)),
			fragment_shader: gfx.resource_manager.compile_shader(CompileShaderRequest::fragment("egui fs", FRAGMENT_SOURCE)),
		})
	}

	pub fn apply_textures(&mut self, _gfx: &mut gfx::System, deltas: &[(TextureId, ImageDelta)]) {
		if deltas.is_empty() {
			return
		}

		println!("Apply {} texture deltas", deltas.len());
	}

	pub fn free_textures(&mut self, _gfx: &mut gfx::System, to_free: &[TextureId]) {
		if to_free.is_empty() {
			return
		}

		println!("Free {} texture deltas", to_free.len());
		
	}

	pub fn paint_triangles(&mut self, gfx: &mut gfx::System, primitives: &[ClippedPrimitive]) {
		if primitives.is_empty() {
			return
		}

		let backbuffer_size = gfx.backbuffer_size();

		let mut group = gfx.frame_encoder.command_group("Paint Egui");

		group.execute(|core, _| {
			unsafe {
				core.gl.Disable(gl::DEPTH_TEST);
				core.gl.Disable(gl::CULL_FACE);
				core.gl.Enable(gl::BLEND);
				
				core.gl.BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
				core.gl.BlendFuncSeparate(
					// egui outputs colors with premultiplied alpha:
					gl::ONE,
					gl::ONE_MINUS_SRC_ALPHA,
					// Less important, but this is technically the correct alpha blend function
					// when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
					gl::ONE_MINUS_DST_ALPHA,
					gl::ONE,
				);
			}
		});

		// TODO(pat.m): are egui coords in logical or physical coordinates?
		// this might be incorrect with scaling
		let transforms = group.upload(&[backbuffer_size]);

		for ClippedPrimitive{clip_rect, primitive} in primitives {
			let Primitive::Mesh(mesh) = primitive else { unimplemented!() };

			#[repr(C)]
			#[derive(Copy, Clone)]
			struct Vertex {
				pos: Vec2,
				uv: [u16; 2],
				color: [u8; 4],
			}

			let vertices = group.upload_iter(mesh.vertices.iter()
				.map(|v| Vertex {
					pos: Vec2::new(v.pos.x, v.pos.y),
					uv: [(v.uv.x * 65535.0) as u16, (v.uv.y * 65535.0) as u16],
					color: v.color.to_array(),
				}));

			group.draw(self.vertex_shader, self.fragment_shader)
				.elements(mesh.indices.len() as u32)
				.indexed(&mesh.indices)
				.ssbo(0, vertices)
				.ubo(0, transforms);
		}
		
		group.execute(|core, _| {
			unsafe {
				core.gl.Enable(gl::DEPTH_TEST);
				// core.gl.Enable(gl::CULL_FACE);
				core.gl.Disable(gl::BLEND);
			}
		});
	}
}