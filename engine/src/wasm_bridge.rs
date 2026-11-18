use once_cell::sync::Lazy;
use std::sync::Mutex;

pub struct SongPayload {
    pub chart: String,
    /// Optional audio. `None` plays the chart silently — the chart clock is
    /// frame-driven, so playback does not depend on audio being present.
    pub audio: Option<Vec<u8>>,
}

/// Load a chart with no audio — plays silently against the frame clock.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_chart(chart: String) {
    push_load(SongPayload { chart, audio: None });
}

/// Load a chart with its audio bytes — the BGM becomes the authoritative
/// clock (see `chart_playback.rs::sync_to_audio_position`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_song(chart: String, audio: Vec<u8>) {
    push_load(SongPayload {
        chart,
        audio: Some(audio),
    });
}

/// Songs go through the command queue, so a load and the transport commands
/// around it apply in the order they were sent.
pub fn push_load(song: SongPayload) {
    COMMANDS.lock().unwrap().push(EngineCommand::Load(song));
}

// ── Transport commands ───────────────────────────────────────────────────────
//
// JS pushes chart loads and transport intents here; the `apply_commands`
// Bevy system drains them each frame, in the order they were sent, and acts on
// the BGM channel + ChartPlayback. The wasm boundary stays a thin,
// lock-guarded mailbox.

pub enum EngineCommand {
    /// A chart, with its music or without (`load_chart`, `load_song`).
    Load(SongPayload),
    Pause,
    Resume,
    Restart,
    /// Jump to a chart time (seconds), playing or paused (visualizer-majdata 08).
    Seek(f64),
    /// Clear the chart, every note on screen and the music
    /// (visualizer-majdata 02): the Play page sends it when it is left.
    Unload,
    SetSongSpeed(f32),
    SetNoteSpeed(f32),
    /// Diagnostics: MSAA sample count on the main camera (1 = off, 2, 4, 8).
    SetMsaa(u32),
    /// Diagnostics: override the window scale factor (`None` = back to the
    /// browser's devicePixelRatio). `Some(1.0)` renders at CSS pixels.
    SetRenderScale(Option<f32>),
    /// How many wall milliseconds ahead of the visual hit guide sounds and
    /// metronome clicks are requested (`systems::audio::AudioLead`).
    SetAudioLeadMs(f32),
}

static COMMANDS: Lazy<Mutex<Vec<EngineCommand>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[cfg(target_arch = "wasm32")]
fn push_command(cmd: EngineCommand) {
    COMMANDS.lock().unwrap().push(cmd);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn pause() {
    push_command(EngineCommand::Pause);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn resume() {
    push_command(EngineCommand::Resume);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn restart() {
    push_command(EngineCommand::Restart);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn seek(secs: f64) {
    push_command(EngineCommand::Seek(secs));
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn unload() {
    request_unload();
}

/// One queue for loads and commands: an unload drops any chart sent before
/// it and none sent after (`apply_commands` applies them in order).
pub fn request_unload() {
    COMMANDS.lock().unwrap().push(EngineCommand::Unload);
}

/// Set the playback speed multiplier (affects both the audio rate and the chart
/// timing). `1.0` is normal speed.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_song_speed(rate: f32) {
    push_command(EngineCommand::SetSongSpeed(rate));
}

/// Set the note speed multiplier (affects the chart timing only). `7.0` is normal speed.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_note_speed(rate: f32) {
    push_command(EngineCommand::SetNoteSpeed(rate));
}

// ── Render diagnostics ───────────────────────────────────────────────────────
//
// Runtime knobs for the mobile frame-pacing investigation
// (mobile-webview-spike 02). They exist so one visit on a phone can A/B the
// GPU-side suspects without a deploy per guess. Same queue as transport.

/// MSAA samples on the main camera: 1 (off), 2, 4 (Bevy's default) or 8.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_msaa(samples: u32) {
    push_command(EngineCommand::SetMsaa(samples));
}

/// Render scale override. `1.0` draws at CSS pixels regardless of
/// devicePixelRatio; `0` restores the browser's own ratio.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_render_scale(scale: f32) {
    let override_ = if scale > 0.0 { Some(scale) } else { None };
    push_command(EngineCommand::SetRenderScale(override_));
}

/// Audio lead in wall milliseconds: guide sounds and metronome clicks are
/// requested this far ahead of the visual hit, to cancel output latency.
/// `0` is desktop behaviour.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_audio_lead_ms(ms: f32) {
    push_command(EngineCommand::SetAudioLeadMs(ms.max(0.0)));
}

pub fn take_commands() -> Vec<EngineCommand> {
    std::mem::take(&mut *COMMANDS.lock().unwrap())
}

// ── Diagnostics outbox ───────────────────────────────────────────────────────
//
// The one message that flows the other way. `systems::frame_stats` writes a
// JSON snapshot here once a second; JS polls it. Still a lock-guarded mailbox —
// Bevy is never called into — just read instead of drained.

static FRAME_STATS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

// ── Playback readout (visualizer-majdata 09) ────────────────────────────────
//
// Published every frame, read by the page's timeline at its own frame rate.
// The chart id rises with every load and unload, so the page knows when to
// read the timeline again.

/// `(position in chart seconds, playing, chart id)`.
static PLAYBACK: Lazy<Mutex<(f64, bool, u32)>> = Lazy::new(|| Mutex::new((0.0, false, 0)));
/// The loaded chart's `Timeline::to_json`, `""` when none.
static TIMELINE: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

pub fn publish_playback(position: f64, playing: bool) {
    let mut snapshot = PLAYBACK.lock().unwrap();
    snapshot.0 = position;
    snapshot.1 = playing;
}

/// A chart was loaded (its timeline) or unloaded (`""`).
pub fn bump_chart(timeline_json: String) {
    let mut snapshot = PLAYBACK.lock().unwrap();
    snapshot.2 = snapshot.2.wrapping_add(1);
    *TIMELINE.lock().unwrap() = timeline_json;
}

pub fn playback_snapshot() -> (f64, bool, u32) {
    *PLAYBACK.lock().unwrap()
}

pub fn timeline_json() -> String {
    TIMELINE.lock().unwrap().clone()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn playback_position() -> f64 {
    playback_snapshot().0
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn playback_playing() -> bool {
    playback_snapshot().1
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn chart_id() -> u32 {
    playback_snapshot().2
}

/// The loaded chart's timeline as JSON: `length` (seconds) and `measures`,
/// each `start` (seconds) and its `tap`/`slide`/`touch`/`hold` counts; `""`
/// when no chart is loaded.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn timeline() -> String {
    timeline_json()
}

pub fn publish_frame_stats(json: String) {
    *FRAME_STATS.lock().unwrap() = json;
}

/// The latest frame-pacing snapshot as JSON, or `""` before the first second
/// has elapsed. Fields: `fps`, `avg_ms`, `max_ms`, `frames`, `slow_frames`
/// (>33ms), `stalled_frames` (>100ms), `canvas` (physical pixels),
/// `scale_factor`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn frame_stats() -> String {
    FRAME_STATS.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads and transport commands share one queue, so they apply in the
    /// order JS sent them: a gesture-less load is followed by a pause that
    /// must win, and an unload must win over a load sent before it, but not
    /// over one sent after.
    #[test]
    fn loads_and_commands_drain_in_the_order_sent() {
        let chart = || SongPayload {
            chart: "(120){4}1,E".into(),
            audio: None,
        };
        push_load(chart());
        COMMANDS.lock().unwrap().push(EngineCommand::Pause);
        request_unload();
        push_load(chart());
        assert!(matches!(
            take_commands().as_slice(),
            [
                EngineCommand::Load(_),
                EngineCommand::Pause,
                EngineCommand::Unload,
                EngineCommand::Load(_),
            ]
        ));
    }

    #[test]
    fn a_new_chart_bumps_the_id_and_carries_its_timeline() {
        let (_, _, before) = playback_snapshot();
        bump_chart(r#"{"length":1.000,"measures":[]}"#.into());
        bump_chart(r#"{"length":2.000,"measures":[]}"#.into());
        publish_playback(1.5, false);
        assert_eq!(playback_snapshot(), (1.5, false, before + 2));
        assert_eq!(timeline_json(), r#"{"length":2.000,"measures":[]}"#);
    }
}
