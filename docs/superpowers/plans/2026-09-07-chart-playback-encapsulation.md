# ChartPlayback Encapsulation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `engine/src/systems/chart_playback.rs`'s `ChartPlayback` resource own its invariants instead of exposing five public fields that five call sites across four files mutate directly — collapse two multi-field mutation sequences (chart-load reset, restart) into single methods so the invariant "these fields always change together" becomes structurally true instead of true-by-convention, and move the `elapsed_time += delta * chart_speed` scaling formula from `tick_clock` into `ChartPlayback` itself.

**Architecture:** `ChartPlayback`'s five fields (`timed_events`, `next_spawn_index`, `note_speed`, `chart_speed`, `is_playing`, `elapsed_time`) become module-private. A new method surface (`load_chart`, `restart`, `pause`, `resume`, `tick`, `sync_to_audio_position`, `set_chart_speed`, `set_note_speed`, plus getters `chart_speed()`, `note_speed()`, `is_playing()`, `has_events()`, `is_finished()`, `timed_events()`) replaces every raw read/write. `advance()`/`approach_time()` are unchanged — they're already proper methods. Because Rust field privacy is enforced at compile time across the whole crate in one shot, this is a single atomic change: every call site must move to the new methods together, or the crate won't build. There is no way to split this into independently-buildable sub-tasks the way a TDD red/green cycle normally would — so this plan is one task with many small steps, verified once at the end via a full build + manual smoke test, per your explicit decision to defer automated engine tests to a separate future effort.

**Tech Stack:** Rust, Bevy ECS (`Res`/`ResMut` resource pattern) — no new dependencies.

**Spec:** No separate spec document — this plan encodes the design reached via `/grilling` in-session (candidate 2 of the 2026-09-07 architecture review): bundle `load_chart`/`restart` into invariant-preserving methods, split `elapsed_time` updates into `tick()`/`sync_to_audio_position()` so the `chart_speed` scaling formula lives inside `ChartPlayback`, make all five fields private, and — per your explicit call — skip adding tests now; full `engine/` test coverage is a separate future effort.

## Global Constraints

- No automated tests in this plan (explicit decision during grilling — `engine/` as a whole gets tested later, as its own effort). Verification is `cargo build -p maiscope-viewer` (compiler-enforced correctness of every call site) plus one manual end-to-end smoke test through the real visualizer page.
- Behavior must not change — this is a pure encapsulation refactor. Every formula (the `chart_speed` scaling in the frame-driven clock branch, the `approach_time` lead calculation, the metronome's rate/phase math) must produce bit-identical results to today.
- `advance()` and `approach_time()` keep their existing signatures — they're already the deep, well-shaped part of this resource's interface; don't touch them beyond making the fields they read private (they're methods on `ChartPlayback` already, so they need no changes at all).
- Never touch production — this is local-only engine/wasm work; no production target exists (per root CLAUDE.md).

---

### Task 1: Encapsulate `ChartPlayback` and migrate every call site

**Files:**
- Modify: `engine/src/systems/chart_playback.rs` (full rewrite of the struct + impl block)
- Modify: `engine/src/systems/mod.rs:23-32,38-68,70-112` (`check_if_ready`, `tick_clock`, `ingest_songs`)
- Modify: `engine/src/systems/visual/spawning.rs:33-119` (`next_event`, `apply_commands`)
- Modify: `engine/src/systems/audio.rs:88-138,140-162` (`tick_metronome`, `start_bgm`)
- Modify: `engine/src/systems/visual/movement/mod.rs:187-199` (the movement-speed system whose signature starts at line 187 — read `chart.note_speed`/`chart.chart_speed` at lines 197-198)

**Interfaces:**
- Consumes: `TimedEvent` (`engine/src/systems/component.rs:11-15`, unchanged: `{ time: f64, event: ChartEvent, bpm: f32 }`, already `#[derive(Debug, Clone)]`), `ChartEvent` (unchanged).
- Produces (the entire public method surface every other file in this task uses):
  - `pub fn load_chart(&mut self, events: Vec<ChartEvent>)`
  - `pub fn restart(&mut self)`
  - `pub fn pause(&mut self)`
  - `pub fn resume(&mut self)`
  - `pub fn tick(&mut self, delta_wall_secs: f64)`
  - `pub fn sync_to_audio_position(&mut self, position: f64)`
  - `pub fn set_chart_speed(&mut self, rate: f32)`
  - `pub fn set_note_speed(&mut self, rate: f32)`
  - `pub fn chart_speed(&self) -> f32`
  - `pub fn note_speed(&self) -> f32`
  - `pub fn is_playing(&self) -> bool`
  - `pub fn has_events(&self) -> bool`
  - `pub fn is_finished(&self) -> bool`
  - `pub fn timed_events(&self) -> &[TimedEvent]`
  - `pub fn advance(&mut self) -> Option<TimedEvent>` (unchanged, already existed)
  - `pub fn approach_time(&self) -> f64` (unchanged, already existed)

- [ ] **Step 1: Rewrite `chart_playback.rs` — private fields, new method surface**

Replace the entire file:

```rust
use super::component::{ChartEvent, TimedEvent};
use super::{DEFAULT_BPM, GROWING, MOVING};
use bevy::prelude::*;

#[derive(Resource)]
pub struct ChartPlayback {
    /// Pre-computed list of timed events with absolute timestamps.
    timed_events: Vec<TimedEvent>,
    /// Index of the next timed event to be spawned. Invariant: always stays
    /// in sync with `timed_events` — reset together via `load_chart`/`restart`,
    /// never mutated independently of them.
    next_spawn_index: usize,

    /// Speed of the note (visual travel speed multiplier).
    note_speed: f32,
    /// Playing speed (chart speed multiplier, affects both timing and visuals).
    chart_speed: f32,
    /// Is playing.
    is_playing: bool,
    /// Elapsed playback time in seconds (advances each frame).
    elapsed_time: f64,
}

impl Default for ChartPlayback {
    fn default() -> Self {
        Self {
            timed_events: Vec::new(),
            next_spawn_index: 0,
            note_speed: 7.0,
            chart_speed: 1.0,
            is_playing: true,
            elapsed_time: 0.0,
        }
    }
}

impl ChartPlayback {
    /// Pre-compute absolute timestamps for every event in the parsed chart.
    ///
    /// The simai timing model:
    /// - `(BPM)` sets the current BPM.
    /// - `{N}` sets the length divider (resolution).
    /// - `{#S}` sets the per-comma length directly to S seconds (absolute length mode).
    /// - Per-comma length = `240 / BPM / resolution` (seconds), unless absolute length is active.
    /// - Each comma (`Rest` or `NoteGroup`) advances time by the per-comma length.
    /// - BPM/resolution changes take effect immediately and do NOT advance time.
    fn compute_timestamps(&mut self, events: Vec<ChartEvent>) {
        let mut timed: Vec<TimedEvent> = Vec::with_capacity(events.len());
        let mut current_time: f64 = 0.0;
        let mut bpm: f32 = 240.0;
        let mut resolution: u32 = 4;
        // When `{#S}` is active, per-comma length is set directly in seconds.
        // A subsequent `{N}` (resolution change) clears this and reverts to
        // the normal `240 / BPM / resolution` formula.
        let mut absolute_comma_length: Option<f64> = None;

        for event in events {
            match &event {
                ChartEvent::BpmChange(new_bpm) => {
                    bpm = *new_bpm;
                }
                ChartEvent::ResolutionChange(new_res) => {
                    resolution = *new_res;
                    absolute_comma_length = None;
                }
                ChartEvent::AbsoluteLength(seconds) => {
                    absolute_comma_length = Some(*seconds);
                }
                ChartEvent::NoteGroup(_) | ChartEvent::Rest => {
                    timed.push(TimedEvent {
                        time: current_time,
                        event: event.clone(),
                        bpm,
                    });
                    let per_comma = absolute_comma_length
                        .unwrap_or_else(|| DEFAULT_BPM as f64 / bpm as f64 / resolution as f64);
                    current_time += per_comma;
                }
            }
        }

        self.timed_events = timed;
        self.next_spawn_index = 0;
    }

    /// Load a freshly parsed chart: pre-computes timestamps and hard-resets
    /// playback state. Loading a new chart must flush the previous one's
    /// position, so `next_spawn_index`/`elapsed_time` reset and playback
    /// (re)starts — callers used to do this as three separate field writes
    /// (`compute_timestamps` + `elapsed_time = 0.0` + `is_playing = true`);
    /// bundling them here makes forgetting one impossible.
    pub fn load_chart(&mut self, events: Vec<ChartEvent>) {
        self.compute_timestamps(events);
        self.elapsed_time = 0.0;
        self.is_playing = true;
    }

    /// Rewind to the start of the current chart and resume playing. Does not
    /// touch the audio instance or spawned note entities — those are a Bevy
    /// system's concern (see `apply_commands`), not this resource's.
    pub fn restart(&mut self) {
        self.next_spawn_index = 0;
        self.elapsed_time = 0.0;
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        self.is_playing = true;
    }

    /// Advance the clock by `delta_wall_secs` of real time, scaled by the
    /// current chart speed. Used when no BGM is present — the frame-driven
    /// path (see `sync_to_audio_position` for the BGM-slaved path).
    pub fn tick(&mut self, delta_wall_secs: f64) {
        self.elapsed_time += delta_wall_secs * self.chart_speed as f64;
    }

    /// Slave the clock to an external audio position (chart-time seconds).
    /// Used when a BGM is present — the BGM's playback position already
    /// equals chart-time (it plays at `chart_speed`), so this is a direct
    /// assignment, not a scaled increment.
    pub fn sync_to_audio_position(&mut self, position: f64) {
        self.elapsed_time = position;
    }

    pub fn set_chart_speed(&mut self, rate: f32) {
        self.chart_speed = rate;
    }

    pub fn set_note_speed(&mut self, rate: f32) {
        self.note_speed = rate;
    }

    pub fn chart_speed(&self) -> f32 {
        self.chart_speed
    }

    pub fn note_speed(&self) -> f32 {
        self.note_speed
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Whether a chart has been loaded (has any events at all).
    pub fn has_events(&self) -> bool {
        !self.timed_events.is_empty()
    }

    /// Whether playback has advanced past the last event.
    pub fn is_finished(&self) -> bool {
        self.next_spawn_index >= self.timed_events.len()
    }

    /// Read-only access to the full event list, e.g. for the metronome's BPM
    /// cursor, which needs to scan events without advancing the spawn index.
    pub fn timed_events(&self) -> &[TimedEvent] {
        &self.timed_events
    }

    /// How far in advance (in chart-time seconds) notes spawn before their hit.
    ///
    /// A note travels in `(GROWING + MOVING) / (chart_speed · note_speed)`
    /// **wall** seconds. The clock advances at `chart_speed` (chart-time per wall
    /// second), so the lead in chart-time — the units of `elapsed_time` /
    /// `event.time` — is `travel_wall · chart_speed`, and the `chart_speed`
    /// cancels: the lead is purely `(GROWING + MOVING) / note_speed`.
    pub fn approach_time(&self) -> f64 {
        (GROWING + MOVING) / self.note_speed as f64
    }

    /// Checks whether the next event is due based on the current elapsed time,
    /// and if so, advances the spawn index and returns it.
    ///
    /// This is the real-time playback path — `elapsed_time` is NOT mutated here;
    /// it is driven externally by the frame loop.
    pub fn advance(&mut self) -> Option<TimedEvent> {
        let event = self.timed_events.get(self.next_spawn_index)?;

        if self.elapsed_time + self.approach_time() >= event.time {
            self.next_spawn_index += 1;
            Some(event.clone())
        } else {
            None
        }
    }
}
```

Note what changed from the original: `#![allow(dead_code)]` is gone (every field is now read by a method in this same `impl` block, so nothing is flagged dead); `compute_timestamps` lost its `pub` (it's now purely an internal helper for `load_chart`).

- [ ] **Step 2: Update `systems/mod.rs`**

```diff
 fn check_if_ready(
     playback: Res<chart_playback::ChartPlayback>,
     mut next_state: ResMut<NextState<AppState>>,
 ) {
     // The chart clock is frame-driven and audio is optional, so readiness only
     // depends on having a parsed chart. As soon as it has events, start playing.
-    if !playback.timed_events.is_empty() {
+    if playback.has_events() {
         next_state.set(AppState::Playing);
     }
 }
```

```diff
 fn tick_clock(
     time: Res<Time>,
     mut playback: ResMut<ChartPlayback>,
     bgm_instance: Option<Res<audio::BgmInstance>>,
     audio_instances: Res<Assets<AudioInstance>>,
 ) {
-    if !playback.is_playing {
+    if !playback.is_playing() {
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
-                | PlaybackState::Paused { position } => playback.elapsed_time = position,
+                | PlaybackState::Paused { position } => playback.sync_to_audio_position(position),
                 _ => {}
             }
         }
         return;
     }

     // No BGM (chart loaded silently): drive the clock off frame time.
-    playback.elapsed_time += time.delta_secs_f64() * playback.chart_speed as f64;
+    playback.tick(time.delta_secs_f64());
 }
```

```diff
         // Hard reset: loading a new chart must flush the previous one. Despawn
         // every live note and rewind the clock, otherwise old notes linger and
         // the new chart spawns against a stale `elapsed_time`.
         for entity in &notes {
             commands.entity(entity).despawn();
         }
-        playback.compute_timestamps(events); // resets next_spawn_index
-        playback.elapsed_time = 0.0;
-        playback.is_playing = true;
+        playback.load_chart(events);
```

- [ ] **Step 3: Update `systems/visual/spawning.rs`**

```diff
 pub fn next_event(
     mut commands: Commands,
     mut chart: ResMut<ChartPlayback>,
     note_assets: Res<NoteAssets>,
     layout: Res<ButtonLayout>,
 ) {
-    if !chart.is_playing {
+    if !chart.is_playing() {
         return;
     }

     // `chart.elapsed_time` is maintained by `tick_clock`: slaved to the BGM's
     // playback position when a song is present (drift-free), or frame-driven when
     // the chart plays silently. Either way spawning just reads `elapsed_time`.
     //
     // .map() clones event data and releases the &mut borrow on chart,
-    // so chart.chart_speed / chart.note_speed are accessible in the loop body.
+    // so chart.chart_speed() / chart.note_speed() are accessible in the loop body.
     while let Some((event, bpm)) = chart.advance().map(|e| (e.event.clone(), e.bpm)) {
         if let ChartEvent::NoteGroup(notes) = event {
             let is_paired = notes.len() >= 2;
             for note in &notes {
                 spawn_note(
                     &mut commands,
                     note,
                     is_paired,
                     bpm,
-                    chart.chart_speed,
-                    chart.note_speed,
+                    chart.chart_speed(),
+                    chart.note_speed(),
                     &note_assets,
                     &layout,
                 );
             }
         }
     }

-    if chart.next_spawn_index >= chart.timed_events.len() {
-        chart.is_playing = false;
+    if chart.is_finished() {
+        chart.pause();
     }
 }
```

```diff
     for cmd in crate::wasm_bridge::take_commands() {
         match cmd {
             crate::wasm_bridge::EngineCommand::Pause => {
                 bgm_channel.pause();
-                chart.is_playing = false;
+                chart.pause();
             }
             crate::wasm_bridge::EngineCommand::Resume => {
                 bgm_channel.resume();
-                chart.is_playing = true;
+                chart.resume();
             }
             crate::wasm_bridge::EngineCommand::SetSongSpeed(rate) => {
                 bgm_channel.set_playback_rate(rate as f64);
-                chart.chart_speed = rate;
+                chart.set_chart_speed(rate);
             }
-            crate::wasm_bridge::EngineCommand::SetNoteSpeed(speed) => chart.note_speed = speed,
+            crate::wasm_bridge::EngineCommand::SetNoteSpeed(speed) => chart.set_note_speed(speed),
             crate::wasm_bridge::EngineCommand::Restart => {
                 // Rewind the audio (the clock anchor) and clear every spawned note.
                 if let Some(instance) = bgm_instance
                     .as_ref()
                     .and_then(|h| audio_instances.get_mut(&h.0))
                 {
                     instance.seek_to(0.0);
                 }
                 for entity in &notes {
                     commands.entity(entity).despawn();
                 }
-                chart.next_spawn_index = 0;
-                chart.elapsed_time = 0.0;
-                chart.is_playing = true;
+                chart.restart();
                 bgm_channel.resume();
             }
         }
     }
```

- [ ] **Step 4: Update `systems/audio.rs`**

```diff
 pub fn tick_metronome(
     chart: Res<crate::systems::chart_playback::ChartPlayback>,
     mut metro: ResMut<Metronome>,
     sfx_channel: Res<AudioChannel<Sfx>>,
     metronome_sfx: Option<Res<MetronomeSfx>>,
 ) {
-    if !metro.enabled || !chart.is_playing {
+    if !metro.enabled || !chart.is_playing() {
         return;
     }
     let Some(sfx) = metronome_sfx else { return };
-    let events = &chart.timed_events;
+    let events = chart.timed_events();
     if events.is_empty() {
         return;
     }

-    let elapsed = chart.elapsed_time;
+    let elapsed = chart.elapsed_time();
     let delta = elapsed - metro.last_elapsed;
     metro.last_elapsed = elapsed;
```

```diff
-    let chart_speed = chart.chart_speed.max(0.01) as f64;
+    let chart_speed = chart.chart_speed().max(0.01) as f64;
```

```diff
     let handle = bgm_channel
         .play(bgm_source.0.clone())
         .with_volume(BGM_VOLUME)
-        .with_playback_rate(chart.chart_speed as f64)
+        .with_playback_rate(chart.chart_speed() as f64)
         .handle();
```

`tick_metronome` reads `chart.elapsed_time` directly, which Task 1's method list doesn't include yet — add one more read-only getter to `chart_playback.rs` in Step 1 before proceeding:

```rust
    pub fn elapsed_time(&self) -> f64 {
        self.elapsed_time
    }
```

(Insert this next to the other getters, e.g. right after `is_playing(&self)`.)

- [ ] **Step 5: Update `systems/visual/movement/mod.rs`**

```diff
     // Travel is faster with both play speed and note speed; hold/slide musical
     // durations follow the play tempo (/ chart_speed), so effective bpm is
     // bpm × chart_speed.
-    let note_speed = chart.note_speed;
-    let chart_speed = chart.chart_speed;
+    let note_speed = chart.note_speed();
+    let chart_speed = chart.chart_speed();
```

- [ ] **Step 6: Full workspace build**

Run: `cd /home/tuthanhh/Coding/atelier/maiscope && cargo build -p maiscope-viewer`
Expected: compiles cleanly, zero errors, zero warnings about the fields that used to need `#[allow(dead_code)]`. If the compiler reports any remaining raw field access (e.g. `chart.is_playing` instead of `chart.is_playing()`, missing parens), it will name the exact file:line — fix it; this is the "the compiler enforces every call site" payoff this refactor is for.

- [ ] **Step 7: End-to-end manual smoke test through the real visualizer**

This refactor touches the actual realtime playback path, so verify behavior is unchanged by running it for real, not just compiling it:

```bash
cd /home/tuthanhh/Coding/atelier/maiscope
./engine/build-wasm.sh
cd apps/host && pnpm dev
```

In the browser: open the visualizer page for any sheet with a chart, and check:
1. Notes spawn and scroll in sync with the (silent or audio) clock — no visual stutter or timing jump vs. before this change.
2. Pause → notes freeze, resume → they continue from the same position (exercises `chart.pause()`/`chart.resume()`).
3. Change note speed and song speed sliders (if wired in the UI) or call `set_note_speed`/`set_song_speed` from the browser console (`window.mod.set_note_speed(10)` etc., matching whatever the wasm module is bound to) — notes visibly respond.
4. Restart — notes clear and chart replays from t=0 (exercises `chart.restart()`, including the `next_spawn_index`/`elapsed_time` reset staying atomic).
5. Let a short chart play to completion — it should stop cleanly (exercises `is_finished()` + `pause()`), no panic, no runaway spawn attempt past the last event.
6. If a metronome/click sound is enabled, confirm it stays on the beat through a restart (exercises `tick_metronome`'s `chart.elapsed_time()`/`chart.timed_events()` reads plus its own rewind-detection logic, which is unchanged).

Expected: all six behaviors match pre-refactor behavior exactly — this is a pure encapsulation change, nothing here should look or feel different.

- [ ] **Step 8: Commit**

```bash
git add engine/src/systems/chart_playback.rs engine/src/systems/mod.rs engine/src/systems/visual/spawning.rs engine/src/systems/audio.rs engine/src/systems/visual/movement/mod.rs
git commit -m "refactor(engine): encapsulate ChartPlayback behind named methods"
```

---

## Self-Review Notes

- **Spec coverage:** private fields ✓ (Step 1), `load_chart`/`restart` bundling ✓ (Step 1, used in Steps 2-3), `tick`/`sync_to_audio_position` split with `chart_speed` scaling moved inside ✓ (Step 1, used in Step 2), getters for every remaining external read ✓ (Step 1 + the `elapsed_time()` addition surfaced while auditing `audio.rs` in Step 4), no tests added ✓ (explicit constraint, verification is build + manual smoke test instead).
- **Completeness check:** grepped the entire `engine/src` tree for every field name (`chart_speed`, `note_speed`, `elapsed_time`, `next_spawn_index`, `is_playing`, `timed_events`, `compute_timestamps`) before writing this plan — five files touch `ChartPlayback` state (`chart_playback.rs`, `systems/mod.rs`, `spawning.rs`, `audio.rs`, `movement/mod.rs`), not the three originally named in the grilling conversation; `audio.rs` and `movement/mod.rs` were found during file-structure mapping and are included as Steps 4-5.
- **Type/name consistency:** every method named in "Produces" above is defined once in Step 1 and referenced by the exact same name in Steps 2-5 — `is_finished()`, `has_events()`, `timed_events()`, `elapsed_time()`, `chart_speed()`, `note_speed()`, `is_playing()`, `pause()`, `resume()`, `restart()`, `load_chart()`, `tick()`, `sync_to_audio_position()`, `set_chart_speed()`, `set_note_speed()`.
- **No placeholders:** every diff above is the literal before/after text; no "similar to Step N" shorthand.

---

**Plan complete and saved to `docs/superpowers/plans/2026-09-07-chart-playback-encapsulation.md`.** Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
