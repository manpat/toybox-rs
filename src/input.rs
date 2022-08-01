//! Everything relating to tracking and translating raw input into usable events.
//! The entry point of this module is [InputSystem].

pub mod action;
pub mod context;
pub mod context_group;
pub mod context_macro;
pub mod frame_state;
pub mod raw;
pub mod system;

pub use system::InputSystem;
pub use frame_state::FrameState;
pub use raw::{MouseButton, Scancode, Keycode, Button};
pub use action::*;
pub use context::{ContextID, InputContext};
pub use context_group::{ContextGroupID, ContextGroup};

// https://www.gamedev.net/tutorials/_/technical/game-programming/designing-a-robust-input-handling-system-for-games-r2975/
