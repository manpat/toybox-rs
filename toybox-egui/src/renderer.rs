use toybox_gfx as gfx;
use crate::prelude::*;
use gfx::prelude::*;

use egui::ClippedPrimitive;
use epaint::Primitive;

use gfx::resource_manager::*;

use crate::textures::TextureManager;


const VERTEX_SOURCE: &str = include_str!("egui.vs.glsl");
const FRAGMENT_SOURCE: &str = include_str!("egui.fs.glsl");
const TEXT_FRAGMENT_SOURCE: &str = include_str!("egui_text.fs.glsl");


pub struct Renderer {
	vertex_shader: ShaderHandle,
	fragment_shader: ShaderHandle,
	text_fragment_shader: ShaderHandle,
}

impl Renderer {
	pub fn new(gfx: &mut gfx::System) -> Renderer {
		Renderer {
			vertex_shader: gfx.resource_manager.request(CompileShaderRequest::vertex("egui vs", VERTEX_SOURCE)),
			fragment_shader: gfx.resource_manager.request(CompileShaderRequest::fragment("egui fs", FRAGMENT_SOURCE)),
			text_fragment_shader: gfx.resource_manager.request(CompileShaderRequest::fragment("egui text fs", TEXT_FRAGMENT_SOURCE)),
		}
	}

	pub fn paint_triangles(&mut self, gfx: &mut gfx::System, primitives: &[ClippedPrimitive], texture_manager: &TextureManager) {
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

			if !clip_rect.is_positive() {
				continue;
			}

			// NOTE: egui is Y-down
			let clip_rect = [
				clip_rect.left() as i16,
				clip_rect.right() as i16,
				clip_rect.top() as i16,
				clip_rect.bottom() as i16,
			];

			#[repr(C)]
			#[derive(Copy, Clone)]
			struct Vertex {
				pos: Vec2,
				uv: [u16; 2],
				color: [u8; 4],
				clip_rect: [i16; 4],
			}

			let vertices = group.upload_iter(mesh.vertices.iter()
				.map(move |v| Vertex {
					pos: Vec2::new(v.pos.x, v.pos.y),
					uv: [(v.uv.x * 65535.0) as u16, (v.uv.y * 65535.0) as u16],
					color: v.color.to_array(),
					clip_rect,
				}));

			let image_name = texture_manager.image_from_texture_id(mesh.texture_id);

			let fragment_shader = match texture_manager.is_font_image(mesh.texture_id) {
				true => self.text_fragment_shader,
				false => self.fragment_shader,
			};

			group.draw(self.vertex_shader, fragment_shader)
				.elements(mesh.indices.len() as u32)
				.indexed(&mesh.indices)
				.ssbo(0, vertices)
				.ubo(0, transforms)
				.sampled_image(0, image_name, texture_manager.sampler());
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