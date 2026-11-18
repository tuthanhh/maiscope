use once_cell::sync::Lazy;
use std::sync::Mutex;

pub struct SongPayload {
    pub chart: String,
    /// Optional audio. `None` plays the chart silently — the chart clock is
    /// frame-driven, so playback does not depend on audio being present.
    pub audio: Option<Vec<u8>>,
}

static INBOX: Lazy<Mutex<Vec<SongPayload>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Load a chart with no audio — plays silently against the frame clock.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_chart(chart: String) {
    INBOX
        .lock()
        .unwrap()
        .push(SongPayload { chart, audio: None });
}

/// Load a chart with its audio bytes — the BGM becomes the authoritative
/// clock (see `chart_playback.rs::sync_to_audio_position`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_song(chart: String, audio: Vec<u8>) {
    INBOX.lock().unwrap().push(SongPayload {
        chart,
        audio: Some(audio),
    });
}

pub fn take_songs() -> Vec<SongPayload> {
    std::mem::take(&mut *INBOX.lock().unwrap())
}

// ── Transport commands ───────────────────────────────────────────────────────
//
// JS pushes transport intents here; the `apply_commands` Bevy system drains them
// each frame and acts on the BGM channel + ChartPlayback. Same queue pattern as
// the song INBOX so the wasm boundary stays a thin, lock-guarded mailbox.

pub enum EngineCommand {
    Pause,
    Resume,
    Restart,
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
pub fn unload() {
    request_unload();
}

/// Songs and commands drain from separate queues, so the order of the JS
/// calls between them is lost. A chart still queued when the page is left
/// (it arrived while the engine was booting) is dropped here, or it could
/// load after the unload and play behind the next page.
pub fn request_unload() {
    INBOX.lock().unwrap().clear();
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

    /// A chart queued while the engine boots, then the page is left: the
    /// unload must win, whatever order the two queues drain in that frame
    /// (final review, visualizer-majdata piece 1).
    #[test]
    fn unload_drops_a_chart_still_queued() {
        INBOX.lock().unwrap().push(SongPayload {
            chart: "(120){4}1,E".into(),
            audio: None,
        });
        request_unload();
        assert!(take_songs().is_empty());
        assert!(matches!(
            take_commands().as_slice(),
            [EngineCommand::Unload]
        ));
    }
}
