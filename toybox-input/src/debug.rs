use crate::*;


pub fn tracker_ui(ui: &mut egui::Ui, input: &mut System) {
	#[derive(Clone, Default)]
	struct State {
		recently_down_buttons: Vec<(Button, u32)>,
		recently_up_buttons: Vec<(Button, u32)>,
	}

	let state_id = ui.id().with("state");
	let mut state: State = ui.data_mut(|map| std::mem::take(map.get_temp_mut_or_default(state_id)));

	for (_, timer) in state.recently_down_buttons.iter_mut() {
		*timer = timer.saturating_sub(1);
	}
	for (_, timer) in state.recently_up_buttons.iter_mut() {
		*timer = timer.saturating_sub(1);
	}

	state.recently_down_buttons.retain(|(_, timer)| *timer > 0);
	state.recently_up_buttons.retain(|(_, timer)| *timer > 0);

	state.recently_down_buttons.extend(input.tracker.down_buttons.iter().map(|&button| (button, 30)));
	state.recently_up_buttons.extend(input.tracker.up_buttons.iter().map(|&button| (button, 30)));

	ui.horizontal(|ui| {
		ui.label("Active Keys: ");
		for button in input.tracker.active_buttons.iter() {
			ui.label(format!("{button:?}"));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Recently Down Keys: ");
		for (button, _) in state.recently_down_buttons.iter() {
			ui.label(format!("{button:?}"));
		}
	});

	ui.horizontal(|ui| {
		ui.label("Recently Up Keys: ");
		for (button, _) in state.recently_up_buttons.iter() {
			ui.label(format!("{button:?}"));
		}
	});

	ui.data_mut(move |map| map.insert_temp(state_id, state));
}