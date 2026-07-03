use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub fn spawn_judgement_ring(mut commands: Commands, layout: Res<super::resources::ButtonLayout>) {
    let radius = super::RADIUS;

    // Main ring — thin stroked circle
    let ring_path = ShapePath::new().move_to(Vec2::new(radius, 0.0)).arc(
        Vec2::ZERO,
        Vec2::splat(radius),
        std::f32::consts::TAU,
        0.0,
    );

    commands.spawn((
        ShapeBuilder::with(&ring_path)
            .stroke((Color::WHITE, radius * 0.005))
            .build(),
        Transform::default(),
    ));

    // 8 dots evenly spaced around the ring
    let dot_r = radius * 0.02;
    let dot_path = ShapePath::new().move_to(Vec2::new(dot_r, 0.0)).arc(
        Vec2::ZERO,
        Vec2::splat(dot_r),
        std::f32::consts::TAU,
        0.0,
    );

    for i in 0..8 {
        commands.spawn((
            ShapeBuilder::with(&dot_path).fill(Color::WHITE).build(),
            Transform::from_xyz(layout.tap[i][0] * radius, layout.tap[i][1] * radius, 1.0),
        ));
    }
}
