#![feature(array_chunks, array_windows, let_chains)]
#![feature(portable_simd, associated_type_defaults)]
#![feature(slice_as_chunks)]
#![doc = include_str!("../README.md")]

pub mod prelude;

pub mod audio;
pub mod engine;
pub mod gfx;
pub mod imgui_backend;
pub mod input;
pub mod perf;
pub mod utility;
pub mod window;

pub use crate::prelude::*;
