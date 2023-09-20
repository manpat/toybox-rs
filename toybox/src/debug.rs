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
}

pub fn show_menu(ctx: &mut super::Context, app: &mut impl super::App, state: &mut MenuState) {
	use egui::menu;

	egui::TopBottomPanel::top("main_debug_menu")
		.show_animated(&ctx.egui, ctx.show_debug_menu, |ui| {
			menu::bar(ui, |ui| {
				ui.menu_button("Toybox", |ui| {
					show_submenus(ui, state);

					ui.separator();

					if ui.button("Quit").clicked() {
						ctx.wants_quit = true;
					}
				});

				app.customise_debug_menu(ui);
			})
		});

	egui::Window::new("Egui Settings")
		.open(&mut state.egui_settings)
		.show(&ctx.egui, |ui| {
			ctx.egui.settings_ui(ui);
		});

	egui::Window::new("Egui Style")
		.open(&mut state.egui_style)
		.show(&ctx.egui, |ui| {
			ctx.egui.style_ui(ui);
		});

	egui::Window::new("Egui Memory")
		.open(&mut state.egui_memory)
		.show(&ctx.egui, |ui| {
			ctx.egui.memory_ui(ui);
		});

	egui::Window::new("Egui Textures")
		.open(&mut state.egui_textures)
		.show(&ctx.egui, |ui| {
			ctx.egui.texture_ui(ui);
		});

	egui::Window::new("Egui Inspection")
		.open(&mut state.egui_inspection)
		.show(&ctx.egui, |ui| {
			ctx.egui.inspection_ui(ui);
		});

	egui::Window::new("Input Tracker")
		.open(&mut state.input_tracker)
		.show(&ctx.egui, |ui| {
			input::debug::tracker_ui(ui, &mut ctx.input);
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
	});
}