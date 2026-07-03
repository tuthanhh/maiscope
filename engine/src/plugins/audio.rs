use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

/// Max number of sounds the kira mixer can play simultaneously. The default
/// (128) can be exhausted by dense note streams (many overlapping guide-hit
/// SFX tails + the BGM), which makes new hits drop out. Raise the ceiling.
/// `AudioSettings` is consumed by `AudioPlugin`, so it must be inserted first.
const SOUND_CAPACITY: usize = 512;

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(AudioSettings {
        sound_capacity: SOUND_CAPACITY,
    })
    .add_plugins(AudioPlugin);
}
