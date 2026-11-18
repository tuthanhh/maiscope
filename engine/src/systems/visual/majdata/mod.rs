//! MajdataPlay's playfield data (visualizer-majdata 13): the slide layouts
//! and sprite sizes `scripts/extract-majdata.py` wrote to
//! `engine/assets/majdata/`, and the rules that pick a layout for a slide.
//! The data is GPL-3.0, from MajdataPlay; see `engine/assets/majdata/SOURCE.md`.

// Until slides are spawned from it (visualizer-majdata 14).
#![allow(dead_code, unused_imports)]

mod layouts;
mod shapes;

pub use layouts::{Bar, SKIN_SCALE, SlideLayouts, sprite_px};
pub use shapes::{Placement, place, prefab_for, shape_end};
