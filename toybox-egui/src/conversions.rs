use common::*;
use cint::ColorInterop;

pub trait CommonVectorExt {
	fn to_egui_vec2(&self) -> egui::Vec2;
	fn to_egui_pos2(&self) -> egui::Pos2;
}

impl CommonVectorExt for Vec2 {
	fn to_egui_vec2(&self) -> egui::Vec2 {
		self.to_compatible()
	}

	fn to_egui_pos2(&self) -> egui::Pos2 {
		self.to_compatible()
	}
}



pub trait CommonColorExt {
	fn to_egui_rgba(&self) -> egui::Rgba;
	fn to_egui_color32(&self) -> egui::Color32;
}

impl CommonColorExt for Color {
	fn to_egui_rgba(&self) -> egui::Rgba {
		egui::Rgba::from_cint((*self).into())
	}

	fn to_egui_color32(&self) -> egui::Color32 {
		egui::Color32::from_cint((*self).into())
	}
}

// TODO(pat.m): Aabb2
// TODO(pat.m): Aabb2i