use bevy::camera::ScalingMode;
use bevy::prelude::*;

#[derive(Component)]
#[require(Camera2d)]
pub struct MainCamera;

/// World-space size that must always stay visible. The judgement ring has
/// radius 350 (diameter 700); this leaves margin for sparks/slide arrows.
const FIT_EXTENT: f32 = 750.0;

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, initialize_camera);
}

fn initialize_camera(mut commands: Commands) {
    // AutoMin keeps a fixed world region in frame whatever the canvas size /
    // aspect ratio (letterboxing the longer axis), so the playfield always fits
    // and never crops — instead of the default 1:1 world-pixel mapping.
    commands.spawn((
        MainCamera,
        // No multisampling. Measured on a phone (mobile-webview-spike 02, P1):
        // Bevy's default Sample4 cost the whole frame budget — 23 FPS with it,
        // 60 without, and halving the pixel count changed nothing. Nothing
        // here benefits from it: sprites are textures, lyon meshes carry
        // their own anti-aliasing.
        bevy::render::view::Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: FIT_EXTENT,
                min_height: FIT_EXTENT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::view::Msaa;

    /// Multisampling was the whole mobile frame budget: with Bevy's default
    /// `Msaa::Sample4` a phone drew 23 FPS, with it off a locked 60, and
    /// halving the pixel count changed nothing (mobile-webview-spike 02, P1).
    /// Every visible thing is a textured sprite or a lyon mesh with its own
    /// anti-aliasing, so there is nothing for MSAA to smooth. Pinned so a
    /// camera rewrite cannot quietly bring the default back.
    #[test]
    fn main_camera_renders_without_msaa() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, initialize_camera);
        app.update();

        let world = app.world_mut();
        let mut cameras = world.query_filtered::<&Msaa, With<MainCamera>>();
        let msaa = cameras
            .single(world)
            .expect("one MainCamera carrying an explicit Msaa component");
        assert_eq!(*msaa, Msaa::Off);
    }
}
