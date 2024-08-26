use crate::prelude::*;

// https://www.egui.rs/#demo

#[derive(Default, Copy, Clone)]
pub struct MenuState {
	egui_settings: bool,
	egui_style: bool,

	egui_memory: bool,
	egui_textures: bool,
	egui_inspection: bool,

	input_tracker: bool,

	#[cfg(feature="gamepad")]
	input_gamepad: bool,
}

pub fn show_menu(ctx: &mut super::Context, app: &mut impl super::App, state: &mut MenuState) {
	use egui::menu;

	let egui_ctx = &ctx.egui.clone();

	egui::TopBottomPanel::top("main_debug_menu")
		.show_animated(egui_ctx, ctx.show_debug_menu, |ui| {
			menu::bar(ui, |ui| {
				ui.menu_button("Toybox", |ui| {
					show_submenus(ui, state);

					ui.separator();

					if ui.button("Quit").clicked() {
						ctx.wants_quit = true;
					}
				});

				app.customise_debug_menu(ctx, ui);
			})
		});

	egui::Window::new("Egui Settings")
		.open(&mut state.egui_settings)
		.show(egui_ctx, |ui| {
			ctx.egui.settings_ui(ui);
		});

	egui::Window::new("Egui Style")
		.open(&mut state.egui_style)
		.show(egui_ctx, |ui| {
			ctx.egui.style_ui(ui);
		});

	egui::Window::new("Egui Memory")
		.open(&mut state.egui_memory)
		.show(egui_ctx, |ui| {
			ctx.egui.memory_ui(ui);
		});

	egui::Window::new("Egui Textures")
		.open(&mut state.egui_textures)
		.show(egui_ctx, |ui| {
			ctx.egui.texture_ui(ui);
		});

	egui::Window::new("Egui Inspection")
		.open(&mut state.egui_inspection)
		.show(egui_ctx, |ui| {
			ctx.egui.inspection_ui(ui);
		});

	egui::Window::new("Input Tracker")
		.open(&mut state.input_tracker)
		.show(egui_ctx, |ui| {
			input::debug::tracker_ui(ui, &mut ctx.input);
		});

	#[cfg(feature="gamepad")]
	egui::Window::new("Gamepad")
		.open(&mut state.input_gamepad)
		.show(egui_ctx, |ui| {
			input::debug::gamepad_ui(ui, &mut ctx.input);
		});
}

fn show_submenus(ui: &mut egui::Ui, state: &mut MenuState) {
	ui.menu_button("Egui", |ui| {
		ui.toggle_value(&mut state.egui_settings, "Settings");
		ui.toggle_value(&mut state.egui_style, "Style");

		ui.toggle_value(&mut state.egui_memory, "Memory");
		ui.toggle_value(&mut state.egui_textures, "Textures");
		ui.toggle_value(&mut state.egui_inspection, "Inspection");
	});

	ui.menu_button("Input", |ui| {
		ui.toggle_value(&mut state.input_tracker, "Tracker");
		// ui.toggle_value(&mut state.input_gamepad, "Gamepad");
	});
}