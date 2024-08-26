pub use common::{self, rand, math::*};
pub use rand::prelude::*;

pub use toy;

pub use toybox_host as host;
pub use toybox_gfx as gfx;
pub use toybox_cfg as cfg;
pub use toybox_audio as audio;
pub use toybox_input as input;
pub use toybox_egui as egui_backend;
pub use toybox_vfs as vfs;

pub use host::prelude::*;
pub use gfx::prelude::*;
pub use audio::prelude::*;
#[allow(unused_imports)] pub use cfg::prelude::*;
#[allow(unused_imports)] pub use vfs::prelude::*;
#[allow(unused_imports)] pub use input::prelude::*;
pub use egui_backend::prelude::*;


pub use anyhow;

pub use tracing;
#[doc(hidden)]
pub use tracing::instrument;

// Yuck! But needed to avoid conflict with toybox::Context
pub use anyhow::Context as AnyhowContext;
pub use std::error::Error;


pub use mint::{self, IntoMint};
pub use cint::{self, ColorInterop};




// TODO(pat.m): move into common
#[derive(Clone, Debug, Default)]
pub struct Gate {
	state: GateState,
}

impl Gate {
	pub fn new() -> Self {
		Gate { state: GateState::Low }
	}

	pub fn state(&self) -> GateState {
		self.state
	}

	pub fn reset(&mut self) {
		self.state = GateState::Low;
	}

	pub fn update(&mut self, condition: bool) -> GateState {
		use GateState::*;

		self.state = match (condition, self.state) {
			(false, Low | FallingEdge) => Low,
			(true, Low | FallingEdge) => RisingEdge,
			(true, High | RisingEdge) => High,
			(false, High | RisingEdge) => FallingEdge,
		};

		self.state
	}
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum GateState {
	#[default]
	Low,
	RisingEdge,
	High,
	FallingEdge,
}

impl GateState {
	pub fn falling_edge(self) -> bool {
		self == GateState::FallingEdge
	}

	pub fn rising_edge(self) -> bool {
		self == GateState::RisingEdge
	}

	pub fn low(self) -> bool {
		matches!(self, GateState::Low | GateState::FallingEdge)
	}

	pub fn high(self) -> bool {
		matches!(self, GateState::High | GateState::RisingEdge)
	}

	pub fn changed(self) -> bool {
		matches!(self, GateState::FallingEdge | GateState::RisingEdge)
	}
}