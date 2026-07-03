use bevy::prelude::App;
use bevy_prototype_lyon::prelude::ShapePlugin;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(ShapePlugin);
}
