//! MajdataPlay's playfield data (visualizer-majdata 13): the slide layouts
//! and sprite sizes `scripts/extract-majdata.py` wrote to
//! `engine/assets/majdata/`, and the rules that pick a layout for a slide.
//! The data is GPL-3.0, from MajdataPlay; see `engine/assets/majdata/SOURCE.md`.

mod layouts;
mod shapes;

pub use layouts::{SlideLayouts, sprite_px};
pub use shapes::{place, prefab_for, shape_end};
