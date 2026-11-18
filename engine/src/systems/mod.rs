use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

use crate::systems::chart_playback::ChartPlayback;

mod audio;
mod chart_playback;
pub(crate) mod component;
mod frame_stats;
mod timeline;
mod visual;

const GROWING: f64 = 1.5;
const MOVING: f64 = 2.0;
const DEFAULT_BPM: f64 = 240.0;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Playing,
}

fn check_if_ready(
    playback: Res<chart_playback::ChartPlayback>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // The chart clock is frame-driven and audio is optional, so readiness only
    // depends on having a parsed chart. As soon as it has events, start playing.
    if playback.has_events() {
        next_state.set(AppState::Playing);
    }
}

/// Authoritative chart clock: advances `elapsed_time` by frame time while
/// playing. This (not the audio position) drives note spawning, so the chart
/// plays at a steady rate from t=0 with no startup jump, and works with or
/// without audio. Audio, when present, is started alongside and just rides along.
fn tick_clock(
    time: Res<Time>,
    mut playback: ResMut<ChartPlayback>,
    bgm_instance: Option<Res<audio::BgmInstance>>,
    audio_instances: Res<Assets<AudioInstance>>,
) {
    if !playback.is_playing() {
        return;
    }

    // When a BGM is present, the audio is the authoritative clock: its playback
    // position (in source seconds) already equals chart-time, because it plays
    // at `chart_speed`. Slaving `elapsed_time` to it keeps notes locked to the
    // music with zero accumulated drift. While the instance is still queued /
    // loading we hold the clock rather than free-running, so playback starts
    // cleanly at t=0 with no pre-roll jump.
    if let Some(bgm) = &bgm_instance {
        if let Some(instance) = audio_instances.get(&bgm.0) {
            match instance.state() {
                PlaybackState::Playing { position }
                | PlaybackState::Pausing { position }
                | PlaybackState::Paused { position } => playback.sync_to_audio_position(position),
                _ => {}
            }
        }
        return;
    }

    // No BGM (chart loaded silently): drive the clock off frame time.
    playback.tick(time.delta_secs_f64());
}

/// The page's timeline reads where the chart is (`wasm_bridge::playback_position`).
fn publish_playback_state(playback: Res<ChartPlayback>) {
    crate::wasm_bridge::publish_playback(playback.elapsed_time(), playback.is_playing());
}

/// Load one chart (`EngineCommand::Load`, drained by `apply_commands`).
pub(crate) fn ingest_song(
    song: crate::wasm_bridge::SongPayload,
    commands: &mut Commands,
    playback: &mut ChartPlayback,
    guide_scheduler: &mut audio::GuideScheduler,
    audio_sources: &mut Assets<AudioSource>,
    notes: &Query<Entity, With<visual::NoteTiming>>,
) {
    // A malformed chart shouldn't crash the engine (panics abort the whole
    // wasm module) — log and skip it instead.
    let events = match simai::parse_chart(&song.chart) {
        Ok(events) => events,
        Err(err) => {
            error!("chart parse failed, skipping song: {err:?}");
            return;
        }
    };

    // Hard reset: loading a new chart must flush the previous one. Despawn
    // every live note and rewind the clock, otherwise old notes linger and
    // the new chart spawns against a stale `elapsed_time`.
    for entity in notes {
        commands.entity(entity).despawn();
    }
    playback.load_chart(events);
    crate::wasm_bridge::bump_chart(timeline::build_timeline(playback.timed_events()).to_json());
    // The guide-sound cursor indexes the old event list; start it over.
    guide_scheduler.reset();

    // The song BGM is optional — a chart with no audio plays silently. The
    // guide SFX is loaded separately at startup, so hit sounds still play.
    // A decode failure is non-fatal: fall back to the silent frame clock.
    if let Some(audio) = song.audio {
        match StaticSoundData::from_cursor(std::io::Cursor::new(audio)) {
            Ok(sound) => {
                commands
                    .insert_resource(audio::BgmSource(audio_sources.add(AudioSource { sound })));
            }
            Err(err) => warn!("BGM decode failed, playing silently: {err:?}"),
        }
    }
}

pub fn register_systems(app: &mut App) {
    // The core game loop will be defined in this function
    app.init_resource::<visual::resources::ButtonLayout>()
        .init_resource::<visual::resources::NoteAssets>()
        .init_resource::<chart_playback::ChartPlayback>()
        .init_resource::<frame_stats::FrameStats>()
        // Every state, every frame: a stall while loading is still a stall.
        .add_systems(Update, frame_stats::sample_frame)
        .init_resource::<audio::AudioLead>()
        .init_resource::<audio::GuideScheduler>()
        .insert_resource(audio::Metronome {
            enabled: true,
            ..Default::default()
        })
        .init_state::<AppState>()
        .add_message::<audio::PlayGuideSoundMessage>() // Register the event
        .add_audio_channel::<audio::Bgm>() // Register Background channel
        .add_audio_channel::<audio::Sfx>() // Register Sound Effects channel
        .add_systems(
            Startup,
            (visual::spawn_judgement_ring, audio::load_guide_sfx),
        )
        // Runs in every state: the first chart loads during `Loading`, and
        // later re-loads (switching songs) arrive while already `Playing`.
        // Before the clock, so a seek, load or unload shows the same frame.
        .add_systems(Update, visual::apply_commands.before(tick_clock))
        .add_systems(Update, check_if_ready.run_if(in_state(AppState::Loading)))
        .add_systems(OnEnter(AppState::Playing), audio::start_bgm)
        // tick_clock must advance the clock before next_event / metronome /
        // guide scheduler read it.
        .add_systems(
            Update,
            (
                tick_clock,
                visual::next_event,
                audio::tick_metronome,
                audio::schedule_guide_sounds,
                publish_playback_state,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            // After next_event: notes it despawns (a seek, a load) are gone
            // before update_movement could queue work on them — an insert on
            // a despawned entity is an error Bevy reports by panicking.
            (
                visual::update_movement.after(visual::next_event),
                audio::handle_guide_sounds,
            )
                .run_if(in_state(AppState::Playing)),
        );
}
