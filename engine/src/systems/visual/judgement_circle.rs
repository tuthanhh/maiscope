use bevy::prelude::*;

use super::resources::NoteAssets;
use super::shapes::ring_sprite;

/// The judgement ring and its eight key marks: MajdataPlay's `outline.png`
/// (visualizer-majdata 15), centred, at its own proportion to the ring.
pub fn spawn_judgement_ring(mut commands: Commands, assets: Res<NoteAssets>) {
    commands.spawn((ring_sprite(&assets), Transform::default()));
}
