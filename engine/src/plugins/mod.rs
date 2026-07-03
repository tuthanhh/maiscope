mod audio;
mod camera;
mod lyon_shape;
use bevy::{
    asset::{AssetMetaCheck, AssetPlugin},
    prelude::*,
    window::WindowResolution,
};

/// Where the `AssetServer` looks for sprite textures.
///
/// * Native: `"assets"` (relative to the working dir, Bevy's default).
/// * wasm: `"/assets"` — an **origin-absolute** URL so the in-browser fetch
///   resolves to `https://host/assets/...` regardless of which route the
///   visualizer page is served from (a relative `"assets"` would resolve
///   against the page path, e.g. `/visualizer/assets/...`, and 404).
///
/// The host build copies `engine/assets/sprites` into `apps/host/public/assets`
/// (see `engine/build-wasm.sh`) so Vite/Tauri serve it at that URL.
#[cfg(target_arch = "wasm32")]
const ASSET_ROOT: &str = "/assets";
#[cfg(not(target_arch = "wasm32"))]
const ASSET_ROOT: &str = "assets";

pub fn register_plugins(app: &mut App) {
    let resolution: WindowResolution = (1000, 800).into();

    app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.0)))
        .add_plugins((
            // Default plugins with window + asset-root configuration
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ASSET_ROOT.to_string(),
                    // No `.meta` sidecars exist; on wasm the per-asset meta fetch
                    // would 404 and can abort the load. Skip it entirely.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "simai player".into(),
                        resizable: false,
                        resolution,
                        canvas: Some("#bevy".to_owned()),
                        desired_maximum_frame_latency: core::num::NonZero::new(1u32),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
            audio::plugin,
            camera::plugin,
            lyon_shape::plugin,
        ));
}
