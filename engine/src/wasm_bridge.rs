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
    SetSongSpeed(f32),
    SetNoteSpeed(f32),
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

pub fn take_commands() -> Vec<EngineCommand> {
    std::mem::take(&mut *COMMANDS.lock().unwrap())
}
