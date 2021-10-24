use crate::gfx;
use common::math::*;



pub struct ImguiBackend {
	shader: gfx::Shader,
	mesh: gfx::Mesh<imgui::DrawVert>,
	uniforms: gfx::Buffer<Uniforms>,

	font_atlas_key: gfx::TextureKey,

	imgui_frame: Option<imgui::Ui<'static>>,
}


impl ImguiBackend {
	pub(crate) fn new(gfx: &mut gfx::Context) -> Result<ImguiBackend, Box<dyn std::error::Error>> {
		init_imgui();

		let imgui_ctx = imgui_mut();

		let shader = gfx.new_simple_shader(
			include_str!("imgui/imgui.vert.glsl"),
			include_str!("imgui/imgui.frag.glsl")
		)?;

		let mesh = gfx::Mesh::new(gfx);
		let uniforms = gfx.new_buffer(gfx::BufferUsage::Stream);
		let font_atlas_key = build_font_atlas(gfx, imgui_ctx);

		init_imgui_input(imgui_ctx);

		Ok(ImguiBackend {
			shader,
			mesh,
			uniforms,

			font_atlas_key,

			imgui_frame: None,
		})
	}

	pub fn frame(&self) -> &imgui::Ui<'static> {
		self.imgui_frame.as_ref()
			.expect("Called Engine::imgui outside of frame")
	}

	pub(crate) fn on_resize(&mut self, drawable_size: Vec2i, window_size: Vec2i) {
		// Set display size and scale here, since SDL 2 doesn't have
		// any easy way to get the scale factor, and changes in said
		// scale factor
		let drawable_size = drawable_size.to_vec2();
		let window_size = window_size.to_vec2();
		let fb_scale = drawable_size / window_size;

		let imgui_ctx = imgui_mut();
		let io = imgui_ctx.io_mut();
		io.display_size = window_size.to_array();
		io.display_framebuffer_scale = fb_scale.to_array();
	}

	// Returns true if imgui wants sole ownership of input
	pub(crate) fn handle_event(&mut self, event: &sdl2::event::Event) -> bool {
		use sdl2::event::Event;

		let imgui_ctx = imgui_mut();
		let io = imgui_ctx.io_mut();

		match *event {
			Event::MouseWheel { x, y, .. } => {
				io.mouse_wheel = y as f32;
				io.mouse_wheel_h = x as f32;
			}

			Event::MouseButtonDown { mouse_btn, .. } => handle_mouse_button(io, &mouse_btn, true),
			Event::MouseButtonUp { mouse_btn, .. } => handle_mouse_button(io, &mouse_btn, false),
			Event::TextInput { ref text, .. } => text.chars().for_each(|c| io.add_input_character(c)),

			Event::KeyDown { scancode: Some(key), keymod, .. } => {
				io.keys_down[key as usize] = true;
				handle_key_modifier(io, &keymod);
			}

			Event::KeyUp { scancode: Some(key), keymod, .. } => {
				io.keys_down[key as usize] = false;
				handle_key_modifier(io, &keymod);
			}

			Event::MouseMotion { x, y, .. } => {
				io.mouse_pos = [x as f32, y as f32];
			}

			_ => {}
		}

		match event {
			Event::MouseButtonDown{..} | Event::MouseButtonUp{..} | Event::MouseMotion{..} | Event::MouseWheel{..}
				=> io.want_capture_mouse,

			Event::KeyDown{..} | Event::KeyUp{..}
				=> io.want_capture_keyboard,

			Event::TextInput{..} => io.want_text_input,

			_ => false
		}
	}

	pub(crate) fn start_frame(&mut self) {
		let imgui_ctx = imgui_mut();
		let io = imgui_ctx.io_mut();
		io.update_delta_time(std::time::Duration::from_secs_f64(1.0 / 60.0));

		self.imgui_frame = Some(imgui_ctx.frame());
	}

	pub(crate) fn draw(&mut self, gfx: &mut gfx::Context) {
		if let Some(frame) = self.imgui_frame.take() {
			self.draw_internal(gfx, frame.render());
		}
	}

	fn draw_internal(&mut self, gfx: &mut gfx::Context, draw_data: &imgui::DrawData) {
		assert!(std::mem::size_of::<imgui::DrawIdx>() == 2, "Imgui using non 16b indices");

		let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
		let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

		self.uniforms.upload_single(&Uniforms {
			transform: Mat4::ortho(0.0, fb_width, fb_height, 0.0, -10.0, 10.0)
		});

		let mut render_state = gfx.render_state();

		render_state.bind_shader(self.shader);
		render_state.bind_texture(0, self.font_atlas_key);
		render_state.bind_uniform_buffer(0, self.uniforms);

		unsafe {
			gfx::raw::Enable(gfx::raw::BLEND);
			gfx::raw::BlendEquation(gfx::raw::FUNC_ADD);
			gfx::raw::BlendFuncSeparate(
				gfx::raw::SRC_ALPHA,
				gfx::raw::ONE_MINUS_SRC_ALPHA,
				gfx::raw::ONE,
				gfx::raw::ONE_MINUS_SRC_ALPHA,
			);
			gfx::raw::Disable(gfx::raw::CULL_FACE);
			gfx::raw::Disable(gfx::raw::DEPTH_TEST);
			gfx::raw::Disable(gfx::raw::STENCIL_TEST);
			gfx::raw::Enable(gfx::raw::SCISSOR_TEST);
		}

		render_state.bind_vao(self.mesh.vao);

		for draw_list in draw_data.draw_lists() {
			self.mesh.upload_separate(&draw_list.vtx_buffer(), &draw_list.idx_buffer());

			for command in draw_list.commands() {
				use imgui::DrawCmd;

				match command {
					DrawCmd::Elements { count: element_count, cmd_params } => {
						let imgui::DrawCmdParams {
							clip_rect,
							texture_id,
							vtx_offset,
							idx_offset,
						} = cmd_params;

						let clip_off = draw_data.display_pos;
						let scale = draw_data.framebuffer_scale;

						let clip_x1 = (clip_rect[0] - clip_off[0]) * scale[0];
						let clip_y1 = (clip_rect[1] - clip_off[1]) * scale[1];
						let clip_x2 = (clip_rect[2] - clip_off[0]) * scale[0];
						let clip_y2 = (clip_rect[3] - clip_off[1]) * scale[1];

						if clip_x1 >= fb_width || clip_y1 >= fb_height || clip_x2 < 0.0 || clip_y2 < 0.0 {
							continue;
						}

						unsafe {
							gfx::raw::Scissor(
								clip_x1 as i32,
								(fb_height - clip_y2) as i32,
								(clip_x2 - clip_x1) as i32,
								(clip_y2 - clip_y1) as i32,
							);

							// gl.bind_texture(glow::TEXTURE_2D, texture_map.gl_texture(texture_id));
						}

						render_state.draw_indexed(gfx::DrawMode::Triangles,
							gfx::IndexedDrawParams::from(element_count as u32)
								.with_offset(idx_offset as u32)
								.with_base_vertex(vtx_offset as u32)
						);
					},

					DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
						use imgui::internal::RawWrapper;
						callback(draw_list.raw(), raw_cmd)
					},

					DrawCmd::ResetRenderState => {
						render_state.bind_shader(self.shader);
						render_state.bind_texture(0, self.font_atlas_key);
						render_state.bind_uniform_buffer(0, self.uniforms);
						render_state.bind_vao(self.mesh.vao);

						unsafe {
							gfx::raw::Enable(gfx::raw::BLEND);
							gfx::raw::BlendEquation(gfx::raw::FUNC_ADD);
							gfx::raw::BlendFuncSeparate(
								gfx::raw::SRC_ALPHA,
								gfx::raw::ONE_MINUS_SRC_ALPHA,
								gfx::raw::ONE,
								gfx::raw::ONE_MINUS_SRC_ALPHA,
							);
							gfx::raw::Disable(gfx::raw::CULL_FACE);
							gfx::raw::Disable(gfx::raw::DEPTH_TEST);
							gfx::raw::Disable(gfx::raw::STENCIL_TEST);
							gfx::raw::Enable(gfx::raw::SCISSOR_TEST);
						}
					}
				}
			}
		}

		unsafe {
			gfx::raw::Disable(gfx::raw::BLEND);
			gfx::raw::Disable(gfx::raw::CULL_FACE);
			gfx::raw::Enable(gfx::raw::DEPTH_TEST);
			gfx::raw::Disable(gfx::raw::STENCIL_TEST);
			gfx::raw::Disable(gfx::raw::SCISSOR_TEST);
		}
	}
}


impl gfx::Vertex for imgui::DrawVert {
	fn descriptor() -> gfx::Descriptor {
		use gfx::vertex::*;

		static ATTRIBUTES: &'static [Attribute] = &[
			Attribute::new(0, AttributeType::Vec2),
			Attribute::new(8, AttributeType::Vec2),
			Attribute::new(16, AttributeType::Unorm8(4)),
		];

		gfx::Descriptor {
			attributes: ATTRIBUTES,
			size_bytes: std::mem::size_of::<Self>() as u32,
		}
	}
}




#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Uniforms {
	pub transform: Mat4,
}





fn handle_mouse_button(io: &mut imgui::Io, button: &sdl2::mouse::MouseButton, pressed: bool) {
	use sdl2::mouse::MouseButton;

	match button {
		MouseButton::Left => { io.mouse_down[0] = pressed }
		MouseButton::Right => { io.mouse_down[1] = pressed }
		MouseButton::Middle => { io.mouse_down[2] = pressed }
		MouseButton::X1 => { io.mouse_down[3] = pressed }
		MouseButton::X2 => { io.mouse_down[4] = pressed }

		_ => {}
	}
}


fn handle_key_modifier(io: &mut imgui::Io, keymod: &sdl2::keyboard::Mod) {
	use sdl2::keyboard::Mod;

	io.key_shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
	io.key_ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
	io.key_alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
	io.key_super = keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
}



fn build_font_atlas(gfx: &mut gfx::Context, imgui: &mut imgui::Context) -> gfx::TextureKey {
	let mut imgui_fonts = imgui.fonts();
	let atlas_texture = imgui_fonts.build_rgba32_texture();
	let font_atlas_size = Vec2i::new(
		atlas_texture.width as _,
		atlas_texture.height as _
	);

	let font_atlas_key = gfx.new_texture(font_atlas_size, gfx::TextureFormat::srgba());
	let mut font_atlas = gfx.resources().get_mut(font_atlas_key);
	font_atlas.upload_rgba8_raw(atlas_texture.data);
	font_atlas.set_filter(true, true);
	font_atlas_key
}


fn init_imgui_input(imgui: &mut imgui::Context) {
	use imgui::Key;
	use sdl2::keyboard::Scancode;

	let io = imgui.io_mut();

	// io.backend_flags.insert(imgui::BackendFlags::HAS_MOUSE_CURSORS);
	// io.backend_flags.insert(imgui::BackendFlags::HAS_SET_MOUSE_POS);

	io[Key::Tab] = Scancode::Tab as _;
	io[Key::LeftArrow] = Scancode::Left as _;
	io[Key::RightArrow] = Scancode::Right as _;
	io[Key::UpArrow] = Scancode::Up as _;
	io[Key::DownArrow] = Scancode::Down as _;
	io[Key::PageUp] = Scancode::PageUp as _;
	io[Key::PageDown] = Scancode::PageDown as _;
	io[Key::Home] = Scancode::Home as _;
	io[Key::End] = Scancode::End as _;
	io[Key::Insert] = Scancode::Insert as _;
	io[Key::Delete] = Scancode::Delete as _;
	io[Key::Backspace] = Scancode::Backspace as _;
	io[Key::Space] = Scancode::Space as _;
	io[Key::Enter] = Scancode::Return as _;
	io[Key::Escape] = Scancode::Escape as _;
	io[Key::KeyPadEnter] = Scancode::KpEnter as _;
	io[Key::A] = Scancode::A as _;
	io[Key::C] = Scancode::C as _;
	io[Key::V] = Scancode::V as _;
	io[Key::X] = Scancode::X as _;
	io[Key::Y] = Scancode::Y as _;
	io[Key::Z] = Scancode::Z as _;
}

use std::cell::UnsafeCell;

thread_local! {
	static IMGUI_CTX: UnsafeCell<Option<imgui::Context>> = UnsafeCell::new(None);
}

fn init_imgui() {
	IMGUI_CTX.with(|ctx| unsafe {
		if let Some(ctx) = ctx.get().as_mut() {
			*ctx = Some({
				let mut imgui = imgui::Context::create();
				imgui.set_ini_filename(None);
				imgui.set_log_filename(None);

				// setup platform and renderer, and fonts to imgui
				imgui.fonts()
					.add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

				imgui.set_platform_name(Some("toybox".to_owned()));

				imgui.io_mut()
					.backend_flags
					.insert(imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET);

				imgui
			});
		}
	});
}

fn imgui_mut() -> &'static mut imgui::Context {
	IMGUI_CTX.with(|ctx| unsafe {
		ctx.get()
			.as_mut()
			.and_then(Option::as_mut)
			.expect("imgui not initialised")
	})
}
