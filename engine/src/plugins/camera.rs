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
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: FIT_EXTENT,
                min_height: FIT_EXTENT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
