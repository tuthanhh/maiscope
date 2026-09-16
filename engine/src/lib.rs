pub struct AppPlugin;
use bevy::prelude::{App, Plugin};

mod plugins;
mod systems;
pub mod wasm_bridge;

/// The simai parser and the types it produces.
///
/// `systems` is otherwise private; this façade exists so `tests/corpus.rs` can
/// reach `parse_chart`. An integration test links the crate as an external
/// consumer, so anything it touches has to be public. Per-note tests stay inline
/// in `systems/parser/` rather than widening this surface any further.
pub mod chart {
    pub use crate::systems::component::{
        ChartEvent, Duration, Note, NoteKind, SlideSegment, SlideShape,
    };
    pub use crate::systems::parser::parse_chart;
}

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((plugins::register_plugins, systems::register_systems));
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    bevy::prelude::App::new().add_plugins(AppPlugin).run();
}
