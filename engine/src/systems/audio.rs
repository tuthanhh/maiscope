// systems/audio.rs
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

// 1. Derive Message instead of Event
#[derive(Message)]
pub struct PlayGuideSoundMessage;

#[derive(Resource)]
pub struct Bgm;

#[derive(Resource)]
pub struct Sfx;

// Guide-tap SFX baked into the binary so it works in a browser / Tauri webview
// without serving an assets dir. It's ALWAYS loaded (independent of the optional
// song BGM) so hit sounds play even for a chart loaded without audio.
const GUIDE_TAP_SFX: &[u8] = include_bytes!("../../assets/system_sounds/answer.wav");
/// Metronome click, baked in like the guide SFX.
const METRONOME_SFX: &[u8] = include_bytes!("../../assets/system_sounds/clock.wav");

/// BGM is ducked below the hit SFX so the guide taps cut through.
const BGM_VOLUME: f32 = 0.4;
/// Base guide-SFX volume, and how much each extra simultaneous hit adds, capped.
const GUIDE_SFX_VOLUME: f32 = 2.0;
const GUIDE_SFX_PER_HIT: f32 = 0.1;
const GUIDE_SFX_VOLUME_MAX: f32 = 1.5;
/// Metronome click volume.
const METRONOME_VOLUME: f32 = 0.6;
/// Shortest comfortable gap between metronome clicks, in **wall** seconds. The
/// beat interval is doubled (1, 2, 4, 8… beats per click) until it clears this,
/// so fast songs click once per bar and slow songs once per beat — always easy
/// to catch by ear.
const METRONOME_MIN_CLICK_SECS: f64 = 0.5;

/// Always-present guide-tap SFX, decoded at startup.
#[derive(Resource)]
pub struct GuideSfx(pub Handle<AudioSource>);

/// Metronome click SFX, decoded at startup.
#[derive(Resource)]
pub struct MetronomeSfx(pub Handle<AudioSource>);

/// Metronome state. `enabled` gates the feature. The click cadence is tracked as
/// a fractional **phase** (in units of clicks) integrated from the chart clock,
/// so it stays correct across BPM changes and play-speed changes. `bpm_cursor`
/// tracks the active BPM; `last_elapsed` detects a rewind (restart/seek).
#[derive(Resource, Default)]
pub struct Metronome {
    pub enabled: bool,
    pub phase: f64,
    pub bpm_cursor: usize,
    pub last_elapsed: f64,
}

/// The optional song BGM, present only when a chart is loaded with audio.
#[derive(Resource)]
pub struct BgmSource(pub Handle<AudioSource>);

/// Handle to the currently-playing BGM instance, so transport commands
/// (restart/seek) can reach the playing audio.
#[derive(Resource)]
pub struct BgmInstance(pub Handle<AudioInstance>);

/// Decode the embedded guide SFX once at startup so it's available regardless of
/// whether a song BGM is ever loaded.
pub fn load_guide_sfx(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    match StaticSoundData::from_cursor(std::io::Cursor::new(GUIDE_TAP_SFX)) {
        Ok(sound) => {
            commands.insert_resource(GuideSfx(audio_sources.add(AudioSource { sound })));
        }
        // Non-fatal: the game still runs, just without guide hit sounds.
        Err(err) => error!("guide SFX decode failed: {err:?}"),
    }
    match StaticSoundData::from_cursor(std::io::Cursor::new(METRONOME_SFX)) {
        Ok(sound) => {
            commands.insert_resource(MetronomeSfx(audio_sources.add(AudioSource { sound })));
        }
        Err(err) => error!("metronome SFX decode failed: {err:?}"),
    }
}

/// Click once per beat, driven by the chart clock. Beat length = `60 / bpm`
/// chart-time seconds at the active BPM (so it follows BPM changes); because the
/// clock advances at `chart_speed` (and is BGM-slaved when a song plays), the
/// clicks track the play speed and stay locked to the music. Self-resets when
/// the clock rewinds (restart).
pub fn tick_metronome(
    chart: Res<crate::systems::chart_playback::ChartPlayback>,
    mut metro: ResMut<Metronome>,
    sfx_channel: Res<AudioChannel<Sfx>>,
    metronome_sfx: Option<Res<MetronomeSfx>>,
) {
    if !metro.enabled || !chart.is_playing() {
        return;
    }
    let Some(sfx) = metronome_sfx else { return };
    let events = chart.timed_events();
    if events.is_empty() {
        return;
    }

    let elapsed = chart.elapsed_time();
    let delta = elapsed - metro.last_elapsed;
    metro.last_elapsed = elapsed;

    // Rewound (restart/seek): elapsed jumped backward → restart the grid, no click.
    if delta < -1e-3 {
        metro.phase = 0.0;
        metro.bpm_cursor = 0;
        return;
    }

    // Active BPM: advance the cursor to the last event whose time has passed.
    while metro.bpm_cursor + 1 < events.len() && events[metro.bpm_cursor + 1].time <= elapsed {
        metro.bpm_cursor += 1;
    }
    let bpm = events[metro.bpm_cursor].bpm.max(1.0);

    // Click interval (chart-time seconds): a quarter note, doubled until the
    // *wall* gap (chart-time / chart_speed) clears the comfortable minimum.
    let chart_speed = chart.chart_speed().max(0.01) as f64;
    let mut beat = 60.0 / bpm as f64;
    while beat / chart_speed < METRONOME_MIN_CLICK_SECS {
        beat *= 2.0;
    }

    // Integrate the click *rate* (Δ chart-time / interval). A BPM/speed change
    // just changes the rate, so the phase stays tempo-correct with no reschedule.
    // One click per integer crossing (delta is one frame, so at most one).
    let prev = metro.phase;
    metro.phase += delta / beat;
    if metro.phase.floor() > prev.floor() {
        sfx_channel
            .play(sfx.0.clone())
            .with_volume(METRONOME_VOLUME);
    }
}

pub fn start_bgm(
    mut commands: Commands,
    bgm_channel: Res<AudioChannel<Bgm>>,
    // Optional: a chart loaded without audio (load_chart) has no BGM and plays
    // silently against the frame clock.
    bgm_source: Option<Res<BgmSource>>,
    chart: Res<crate::systems::chart_playback::ChartPlayback>,
) {
    let Some(bgm_source) = bgm_source else {
        return;
    };

    // Lower the BGM volume slightly so the guide/hit SFX punch through. The
    // instance handle is stored so transport commands can reach the audio.
    // Honor any speed set before playback began (set_speed command) so the audio
    // rate matches the chart timing from the first frame.
    let handle = bgm_channel
        .play(bgm_source.0.clone())
        .with_volume(BGM_VOLUME)
        .with_playback_rate(chart.chart_speed() as f64)
        .handle();
    commands.insert_resource(BgmInstance(handle));
}

pub fn handle_guide_sounds(
    // 2. Use MessageReader
    mut messages: MessageReader<PlayGuideSoundMessage>,
    sfx_channel: Res<AudioChannel<Sfx>>,
    // Optional: absent if the guide SFX failed to decode at startup.
    guide_sfx: Option<Res<GuideSfx>>,
) {
    let hit_count = messages.read().count();
    if hit_count == 0 {
        return;
    }
    let Some(guide_sfx) = guide_sfx else {
        return;
    };

    // Louder for chords (more simultaneous hits), capped so it never clips.
    let volume_multiplier = 1.0 + (hit_count as f32 - 1.0) * GUIDE_SFX_PER_HIT;
    let final_volume = (GUIDE_SFX_VOLUME * volume_multiplier).min(GUIDE_SFX_VOLUME_MAX);

    sfx_channel
        .play(guide_sfx.0.clone())
        .with_volume(final_volume);
}
