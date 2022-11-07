use crate::prelude::*;


pub struct ImguiBackend {
	shader: gfx::Shader,
	mesh: gfx::Mesh<imgui::DrawVert>,
	uniforms: gfx::Buffer<Uniforms>,

	imgui_frame: Option<imgui::Ui<'static>>,
	input_enabled: bool,
	visible: bool,
}


impl ImguiBackend {
	pub(crate) fn new(gfx: &mut gfx::ResourceContext<'_>) -> Result<ImguiBackend, Box<dyn std::error::Error>> {
		init_imgui();

		let imgui_ctx = imgui_mut();

		let shader = gfx.new_simple_shader(
			include_str!("imgui/imgui.vert.glsl"),
			include_str!("imgui/imgui.frag.glsl")
		)?;

		let mesh = gfx::Mesh::new(gfx);
		let uniforms = gfx.new_buffer(gfx::BufferUsage::Stream);
		let _font_atlas_key = build_font_atlas(gfx, imgui_ctx);

		init_imgui_input(imgui_ctx);

		Ok(ImguiBackend {
			shader,
			mesh,
			uniforms,

			imgui_frame: None,
			input_enabled: false,
			visible: false,
		})
	}

	pub fn set_input_enabled(&mut self, input_enabled: bool) {
		self.input_enabled = input_enabled;
	}

	pub fn set_visible(&mut self, visible: bool) {
		self.visible = visible;
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

		let handle_input = self.visible && self.input_enabled;

		match *event {
			Event::MouseWheel { x, y, .. } => {
				io.mouse_wheel = y as f32;
				io.mouse_wheel_h = x as f32;
			}

			Event::MouseButtonDown { mouse_btn, .. } if handle_input => handle_mouse_button(io, &mouse_btn, true),
			Event::MouseButtonUp { mouse_btn, .. } => handle_mouse_button(io, &mouse_btn, false),
			Event::TextInput { ref text, .. } => text.chars().for_each(|c| io.add_input_character(c)),

			Event::KeyDown { scancode: Some(key), keymod, .. } => if handle_input {
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

		if !handle_input {
			return false;
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

	pub(crate) fn clear_input(&mut self) {
		let imgui_ctx = imgui_mut();
		let io = imgui_ctx.io_mut();

		io.mouse_down.fill(false);
		io.keys_down.fill(false);

		io.key_shift = false;
		io.key_ctrl = false;
		io.key_alt = false;
		io.key_super = false;
	}

	pub(crate) fn start_frame(&mut self) {
		let imgui_ctx = imgui_mut();
		let io = imgui_ctx.io_mut();
		io.update_delta_time(std::time::Duration::from_secs_f64(1.0 / 60.0));

		self.imgui_frame = Some(imgui_ctx.frame());
	}

	#[instrument(skip_all, name="ImguiBackend::draw")]
	pub(crate) fn draw(&mut self, gfx: &mut gfx::DrawContext<'_>) {
		if let Some(frame) = self.imgui_frame.take() {
			let frame_data = frame.render();

			if self.visible {
				self.draw_internal(gfx, frame_data);
			}
		}
	}

	fn setup_state(&self, gfx: &mut gfx::DrawContext<'_>) {
		gfx.bind_shader(self.shader);
		gfx.bind_uniform_buffer(0, self.uniforms);
		gfx.bind_vao(self.mesh.vao);
		gfx.set_wireframe(false);

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

	fn draw_internal(&mut self, gfx: &mut gfx::DrawContext<'_>, draw_data: &imgui::DrawData) {
		assert!(std::mem::size_of::<imgui::DrawIdx>() == 2, "Imgui using non 16b indices");

		let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
		let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

		self.uniforms.upload_single(&Uniforms {
			transform: Mat4::ortho(0.0, fb_width, fb_height, 0.0, -10.0, 10.0)
		});

		let get_parameter = |param| unsafe {
			let mut value = 0;
			gfx::raw::GetIntegerv(param, &mut value);
			value
		};

		let blend_src_rgb = get_parameter(gfx::raw::BLEND_SRC_RGB);
		let blend_dst_rgb = get_parameter(gfx::raw::BLEND_DST_RGB);
		let blend_src_alpha = get_parameter(gfx::raw::BLEND_SRC_ALPHA);
		let blend_dst_alpha = get_parameter(gfx::raw::BLEND_DST_ALPHA);
		let blend_equation_rgb = get_parameter(gfx::raw::BLEND_EQUATION_RGB);
		let blend_equation_alpha = get_parameter(gfx::raw::BLEND_EQUATION_ALPHA);
		let wireframe_mode = get_parameter(gfx::raw::POLYGON_MODE) == gfx::raw::LINE as i32;

		let depth_test_enabled;
		let stencil_test_enabled;
		let cull_face_enabled;
		let blend_enabled;

		unsafe {
			depth_test_enabled = gfx::raw::IsEnabled(gfx::raw::DEPTH_TEST) != 0;
			stencil_test_enabled = gfx::raw::IsEnabled(gfx::raw::STENCIL_TEST) != 0;
			cull_face_enabled = gfx::raw::IsEnabled(gfx::raw::CULL_FACE) != 0;
			blend_enabled = gfx::raw::IsEnabled(gfx::raw::BLEND) != 0;
		}

		self.setup_state(gfx);

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
						}

						let texture_key = gfx::TextureKey::from(slotmap::KeyData::from_ffi(texture_id.id() as u64));
						gfx.bind_texture(0, texture_key);

						gfx.draw_indexed(gfx::DrawMode::Triangles,
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
						self.setup_state(gfx);
					}
				}
			}
		}


		let set_enabled = |param, enabled| unsafe {
			if enabled {
				gfx::raw::Enable(param);
			} else {
				gfx::raw::Disable(param);
			}
		};

		set_enabled(gfx::raw::SCISSOR_TEST, false);
		set_enabled(gfx::raw::BLEND, blend_enabled);
		set_enabled(gfx::raw::CULL_FACE, cull_face_enabled);
		set_enabled(gfx::raw::DEPTH_TEST, depth_test_enabled);
		set_enabled(gfx::raw::STENCIL_TEST, stencil_test_enabled);

		unsafe {
			gfx::raw::BlendEquationSeparate(
				blend_equation_rgb as u32,
				blend_equation_alpha as u32,
			);

			gfx::raw::BlendFuncSeparate(
				blend_src_rgb as u32,
				blend_dst_rgb as u32,
				blend_src_alpha as u32,
				blend_dst_alpha as u32,
			);
		}

		gfx.set_wireframe(wireframe_mode);
	}
}


pub fn texture_key_to_imgui_id(key: gfx::TextureKey) -> imgui::TextureId {
	use slotmap::Key;
	imgui::TextureId::new(key.data().as_ffi() as usize)
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



fn build_font_atlas(gfx: &mut gfx::ResourceContext<'_>, imgui: &mut imgui::Context) -> gfx::TextureKey {
	let mut imgui_fonts = imgui.fonts();
	let atlas_texture = imgui_fonts.build_rgba32_texture();
	let font_atlas_size = Vec2i::new(
		atlas_texture.width as _,
		atlas_texture.height as _
	);

	let font_atlas_key = gfx.new_texture(font_atlas_size, gfx::TextureFormat::srgba());
	let mut font_atlas = gfx.resources.get_mut(font_atlas_key);
	font_atlas.upload_rgba8_raw(atlas_texture.data);
	font_atlas.set_filter(true, true);

	imgui_fonts.tex_id = texture_key_to_imgui_id(font_atlas_key);

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
	assert!(std::mem::size_of::<usize>() == std::mem::size_of::<u64>(),
		"Imgui backend assumes that usize is the same size as u64 to get texture keys into imgui.");

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

				setup_style(imgui.style_mut());

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

fn setup_style(style: &mut imgui::Style) {
	style.use_dark_colors();

	// TODO: better colour scheme
	// use imgui::StyleColor;
	// style[StyleColor::]
}