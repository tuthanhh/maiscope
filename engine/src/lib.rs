pub struct AppPlugin;
use bevy::prelude::{App, Plugin};

mod plugins;
mod systems;
pub mod wasm_bridge;

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
