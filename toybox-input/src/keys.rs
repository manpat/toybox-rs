use crate::*;

pub use PhysicalKey::{
	Digit0, Digit1, Digit2, Digit3, Digit4,
	Digit5, Digit6, Digit7, Digit8, Digit9,

    Backquote,
    Minus, Equal,

	KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
	KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
	KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,

	Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
	Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,

	NumpadAdd, NumpadDivide, NumpadMultiply, NumpadSubtract,
	NumpadDecimal, NumpadEnter,

    // Backslash, IntlBackslash
    BracketLeft,
    BracketRight,
	Comma, Period, Slash,
    Quote,
};

pub use LogicalNamedKey::{
	F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
	Alt, Control, Shift, Tab, Space, Enter, Escape,
	ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
	Home, End,
	PageUp, PageDown,
	Backspace, Delete,
	Insert,

	// TODO(pat.m): media buttons?
};