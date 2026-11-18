# Visualizer piece 2 — timeline and seeking: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A timeline under the playfield — a density graph of the chart by measure, time marks every 30 s, a playhead and a measure slider — that seeks the chart, the notes on screen and the music together.

**Architecture:** Every note's phase becomes a function of chart time. A pure model (`schedule.rs`) answers "which phase is this note in at chart time *t*, and how far through it?"; `update_movement` asks it every frame instead of ticking per-note timers. Seeking is then cheap and exact: set the clock, despawn everything, spawn every note the model says is live at *t*. A second pure model (`timeline.rs`) turns the parsed chart into measures and note counts; the engine publishes it and the playback position through the same polled-snapshot pattern as `frame_stats()`, and a new `VizTimeline` draws it.

**Tech Stack:** Rust (Bevy 0.18, bevy_kira_audio 0.25, wasm-bindgen `=0.2.122`), Vue 3 + TypeScript, Vitest.

**Spec:** [`docs/work/visualizer-majdata/spec.md`](spec.md), piece 2 (with the piece-1 planning change: "every on-screen element is a function of chart time" lands here).

## Global Constraints

- Never `cargo build --workspace` / `cargo test --workspace`; engine: `cargo test -p engine --lib` (needs `libasound2-dev`, `libudev-dev`); wasm: `./scripts/build-wasm.sh`.
- `wasm-bindgen` stays pinned at `=0.2.122`.
- JS reaches Bevy only through the mailbox in `engine/src/wasm_bridge.rs`. Reading a snapshot the engine published (as `frame_stats()` does) is allowed; calling into the `World` is not.
- The parser is the single source of note times: the timeline is built from the same `ChartPlayback::timed_events()` the engine plays.
- A seek never fires guide sounds or metronome clicks for the notes it skipped (spec, Constraints).
- Measures are four beats at the BPM in force; simai has no time signature (spec).
- Timeline colours (spec): **pink** tap (taps and breaks of tap kind), **blue** slide (a slide counts by its star tap), **yellow** touch (touches and touch holds), **orange** hold.
- Timeline keys (spec): arrows step one measure, Page Up/Down ten, Home/End; disabled until a chart is loaded.
- Every new UI string goes into all nine `apps/host/src/locales/*.yaml` files.
- `cargo fmt --check`, `cargo clippy -p engine --target wasm32-unknown-unknown -- -D warnings`, `pnpm test`, `pnpm build` clean before each commit.
- One commit per task, on branch `v2`, message ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes (the maintainer may overrule)

- **Effect lengths move to chart time.** A tap's 0.25 s pop, a touch's 0.05 s flash and 0.25 s burst and a hold's 0.1 s pop were wall seconds; they become chart seconds, so at 0.5× they last twice as long on screen. One clock for everything is what makes a seek exact.
- **A note-speed change moves notes already on screen** to where the new speed puts them, instead of letting them finish at the old speed.
- **A seek does not re-create the slide head's 0.25 s pop.** It is a side effect of the star landing, not a chart note.
- **A slide always counts as a slide** in the graph, including one with a plain tap head (`plain_head`); a `SlideStar` (star with no path) counts as a slide; a `HeadlessSlide` has no star tap and counts nothing.

## Review Focus

1. **Seeking into the middle of a hold or a slide** — the hold body is at its true length, the slide's star is part-way along, and the arrows behind it are gone.
2. **Seeking while paused** — the playfield shows the chart at the new time and stays frozen; play continues from there.
3. **Dragging the timeline back and forth quickly** — no notes left behind, no burst of guide sounds, no clicks, frame rate holds.
4. **A chart with BPM changes** — measure numbers on the slider match the bars above them and the music; the last measure ends at the chart's end.
5. **The chart reaching its end** — the playhead stops at the end, the transport shows "play", and play from there restarts nothing until the user seeks or restarts.

Each line's test lives in the task that owns it (marked "Review Focus n").

---

### Task 1: Chart-time phase model (engine, pure)

**Files:**
- Create: `engine/src/systems/visual/schedule.rs` (with `#[cfg(test)] mod tests`), register in `systems/visual/mod.rs`
- Modify: `engine/src/systems/visual/slide_path/trace.rs` (expose slide timing without geometry)

**Interfaces:**
- Produces:
  - `pub enum Phase { Growing, Moving, Holding, Waiting, Sliding, Dying }` (`Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord` — declaration order is phase order)
  - `pub struct PhaseAt { pub phase: Phase, pub elapsed: f32, pub duration: f32 }` (chart seconds)
  - `pub fn phase_at(kind: &NoteKind, hit: f64, bpm: f32, note_speed: f32, t: f64) -> Option<PhaseAt>` — `None` before the note spawns and after it ends.
  - `pub fn note_end(kind: &NoteKind, hit: f64, bpm: f32) -> f64` — chart time the note's last phase ends.
  - `pub fn slide_span_secs(segments: &[SlideSegment], shared_duration: bool, note_bpm: f32) -> (f32, f32)` in `trace.rs` — `(wait, trace_total)`, the same numbers `build`'s `SlidePath::wait_secs` and `trace_total_secs` give.

Phase spans in chart seconds, `hit` = the note's event time:
- Growing `GROWING / note_speed`, then Moving `MOVING / note_speed`, ending at `hit` (every kind).
- Tap, SlideStar: Dying `0.25`.
- Touch: Holding `0.05`, Dying `0.25`.
- TapHold, TouchHold: Holding `duration.hold_secs(bpm)`, Dying `0.1`.
- Slide, HeadlessSlide: Waiting `wait`, Sliding `trace_total` (from `slide_span_secs`); no Dying (a slide ends when its trace ends).

- [ ] **Step 1: Write the failing tests** in `schedule.rs`:
  - `a_tap_is_growing_then_moving_then_dying`: a `Tap` kind (from `(120){4}1,E`), passed `hit = 2.0`, `note_speed = 7.0` → `phase_at(.., 2.0 - 3.5/7.0 - 0.01)` is `None`; at `2.0 - 2.0/7.0 - 0.01` is `Growing`; at `2.0 - 0.1` is `Moving` with `duration ≈ 2.0/7.0`; at `2.1` is `Dying` with `elapsed ≈ 0.1`; at `2.26` is `None`.
  - `a_hold_holds_for_its_musical_length`: `1h[1:1]` (one bar: 2.0 s) at bpm 120, `hit = 0.0` → at `1.0` `Holding`, `duration ≈ 2.0`, `elapsed ≈ 1.0`; `note_end ≈ 2.1`.
  - `a_slide_waits_then_traces`: `1-5[8:1]` at bpm 120 → at `hit + wait/2` `Waiting`; at `hit + wait + total/2` `Sliding` with `elapsed ≈ total/2`; at `note_end + 0.01` `None`.
  - `slide_span_matches_the_built_path`: for `1-5[8:1]`, `1-3[4:1]`, `1>4-7[8:3]` (multi-segment) and a `[#…]` explicit-wait slide, `slide_span_secs` equals `(path.wait_secs, trace_total_secs(&path))` of the path the spawner builds (use `slide_path`'s existing constructor; the executor finds its name).
- [ ] **Step 2: Run** — `cargo test -p engine --lib schedule` → FAIL (`phase_at` not found).
- [ ] **Step 3: Implement** `slide_span_secs` by lifting the wait/total sums out of the path builder (which then calls it, so the two cannot drift), then `phase_at` and `note_end` from the spans above. `GROWING`/`MOVING` come from `systems/mod.rs`.
- [ ] **Step 4: Run** — `cargo test -p engine --lib` → PASS; clippy for wasm clean.
- [ ] **Step 5: Commit** — `feat(engine): chart-time phase model (visualizer-majdata 05)`

---

### Task 2: Notes follow the chart clock (engine)

**Files:**
- Modify: `engine/src/systems/visual/component.rs` (`HitTime`), `spawning.rs` (`spawn_note` takes the hit time), `movement/mod.rs` (`update_movement`), `chart_playback.rs` (remove `visual_delta` and its two tests — superseded)

**Interfaces:**
- Consumes: `phase_at`, `Phase`, `PhaseAt` (Task 1).
- Produces: `#[derive(Component, Clone, Copy)] pub struct HitTime(pub f64);` on every note entity; `spawn_note(.., hit: f64, ..)` — later tasks spawn through it.

`update_movement` no longer reads `Time`. Per note, each frame: `phase_at(kind, hit, bpm, note_speed, chart.elapsed_time())`.
- `None` → despawn the entity (recursive).
- Same phase as the current `NoteTiming` variant → set that timer's elapsed (`Timer::set_elapsed`) and run the phase's existing body.
- A later phase → run the **entry work** of every phase crossed, in order, then replace `NoteTiming` with the new phase (a `Timer` of `duration` with `elapsed` set) and run its body. Entry work is today's transition code, moved: entering Moving makes a touch visible; entering Holding spawns the halo / `TouchJust` (skipped when the target is Dying — its child would outlive Dying's despawn, which only sees existing children); entering Waiting hides the slide head and, only when `t - hit < 0.25`, spawns the hexagon pop as a `NoteKind::Tap` with `HitTime(hit)` (the model then runs it as a Dying tap); entering Dying is today's first-Dying-frame code.
- The `timer.elapsed().is_zero()` first-frame checks and every `just_finished()` transition go: phase changes are the only trigger.
- `next_event` passes `event.time` as the hit time.

- [ ] **Step 1: Write the failing tests** in `movement/mod.rs`'s test module (reuse the `App` set-up of `a_paused_hold_spawns_its_halo_once`, adding `HitTime` and `NoteBpm` to spawned notes):
  - `a_note_sits_where_the_clock_puts_it`: a tap with `HitTime(2.0)`, chart paused at `elapsed = 2.0 - 1.0/7.0` (note speed 7) → after one update its `NoteTiming` is `Moving` with `fraction ≈ 0.5`.
  - `a_paused_note_stays_put` (keeps Review Focus 2 of piece 1): same tap, three updates with the chart paused → `Transform` identical after each.
  - `a_note_past_its_end_is_gone`: same tap, `elapsed = 3.0` → entity despawned after one update.
  - `a_hold_entered_mid_way_shows_one_halo` (Review Focus 1): a `1h[1:1]` hold with `HitTime(0.0)`, `elapsed = 1.0`, three updates → exactly one `HoldHalo`.
  - Keep `a_paused_hold_spawns_its_halo_once` passing.
- [ ] **Step 2: Run** — `cargo test -p engine --lib movement` → FAIL (`HitTime` not found).
- [ ] **Step 3: Implement** as above.
- [ ] **Step 4: Run** — `cargo test -p engine --lib` → PASS; clippy for wasm clean; `./scripts/build-wasm.sh`.
- [ ] **Step 5: Browser check** — a dense chart at 1.0× and 0.5×: notes, holds, slides, touches and their effects look as before (effects twice as long at 0.5×, by decision); pause freezes everything.
- [ ] **Step 6: Commit** — `refactor(engine): notes follow the chart clock (visualizer-majdata 06)`

---

### Task 3: Timeline model (engine, pure)

**Files:**
- Create: `engine/src/systems/timeline.rs` (with tests), register in `systems/mod.rs`

**Interfaces:**
- Consumes: `note_end` (Task 1).
- Produces:
  - `pub fn beat_at(events: &[TimedEvent], t: f64) -> f64` — beats since chart start, integrating each event's `bpm` until the next event's time (before the first event, the first event's bpm; after the last, the last's).
  - `pub fn time_at_beat(events: &[TimedEvent], beat: f64) -> f64` — its inverse.
  - `pub struct MeasureCounts { pub start: f64, pub tap: u32, pub slide: u32, pub touch: u32, pub hold: u32 }`
  - `pub struct Timeline { pub length: f64, pub measures: Vec<MeasureCounts> }` with `pub fn to_json(&self) -> String` (hand-built like `FrameStats::to_json`): `{"length":L,"measures":[{"start":S,"tap":n,"slide":n,"touch":n,"hold":n},…]}`, times to 3 decimals.
  - `pub fn build_timeline(events: &[TimedEvent]) -> Timeline` — `length` = the latest `note_end` (0 for an empty chart); measure *k* starts at `time_at_beat(4k)`; measures run until one starts at or after `length`; each note counts in the measure containing its event time, by the colour rule in Global Constraints and Decisions.

- [ ] **Step 1: Write the failing tests**:
  - `beats_follow_bpm_changes`: `(120){4}1,1,(240)1,1,E` → `beat_at` at the third note's time is `2.0`, and one second later `6.0`; `time_at_beat(beat_at(x)) ≈ x` for x in 0..4 s.
  - `four_beats_make_a_measure` (Review Focus 4): `(120){4}1,1,1,1,1,E` → measures start at `0.0` and `2.0`; counts tap `4` then `1`.
  - `notes_count_by_kind`: one group `1/2h[4:1]/3-5[8:1]/B1` → tap 1, hold 1, slide 1, touch 1; a `Ch[4:1]` touch hold counts as touch; `1$` (a `SlideStar`) as slide.
  - `the_chart_ends_when_its_last_note_does`: `(120){4}1h[1:1],E` → `length ≈ 2.1` (hold + its 0.1 s pop).
  - `json_shape`: a two-measure timeline serialises to the exact string above.
- [ ] **Step 2: Run** — `cargo test -p engine --lib timeline` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** — `cargo test -p engine --lib` → PASS; clippy clean.
- [ ] **Step 5: Commit** — `feat(engine): timeline model, measures and note counts (visualizer-majdata 07)`

---

### Task 4: `Seek` (engine)

**Files:**
- Modify: `chart_playback.rs`, `wasm_bridge.rs`, `visual/spawning.rs` (`apply_commands`, `next_event`), `audio.rs` (`GuideScheduler`, `Metronome`)

**Interfaces:**
- Consumes: `phase_at` (Task 1), `HitTime`/`spawn_note(.., hit, ..)` (Task 2), `beat_at` (Task 3).
- Produces:
  - `ChartPlayback::seek(&mut self, t: f64)` — clamps `t` to `[0, length]` (`length` = Task 3's, cached at `load_chart`), sets `elapsed_time`, sets `next_spawn_index` to the first event with `time > t + approach_time()`, and records `t` as a pending respawn. Keeps `is_playing`.
  - `ChartPlayback::take_respawn(&mut self) -> Option<f64>`.
  - `GuideScheduler::seek(&mut self, events: &[TimedEvent], horizon: f64)` — `next` = first event with `time > horizon`; `releases` = the release times of holds that start at or before `horizon` and release after it; `last_horizon = horizon`.
  - `Metronome::seek(&mut self, events: &[TimedEvent], horizon: f64, chart_speed: f32)` — `last_elapsed = horizon`; `bpm_cursor` = last event at or before `horizon`; `phase = beat_at(horizon) / k`, where `k` is the click-interval doubling `tick_metronome` would use at that bpm and speed (extract that loop into one function both call).
  - `EngineCommand::Seek(f64)`; `#[wasm_bindgen] pub fn seek(secs: f64)`.

The `Seek` arm in `apply_commands`: despawn every `NoteTiming` entity; `chart.seek(t)`; `bgm instance.seek_to(t)` when a BGM instance exists (BGM position is chart time); `guide_scheduler.seek(events, t + lead)` and `metronome.seek(events, t + lead, speed)` with `lead = AudioLead::chart_secs(speed)`. `next_event`, **before** its `is_playing` check: if `take_respawn()` is `Some(t)`, spawn every note of every event with `time <= t + approach_time()` for which `phase_at(.., t)` is `Some` (so a seek while paused shows the frame at `t` — Review Focus 2).

- [ ] **Step 1: Write the failing tests**:
  - `chart_playback.rs` `seek_resumes_spawning_after_the_visible_window`: `(120){4}1,1,1,1,1,1,1,1,E`, note speed 7, `seek(2.0)` → `elapsed_time() == 2.0`; `advance()` returns no event with `time <= 2.0 + 0.5`.
  - `audio.rs` `a_seek_skips_the_guide_sounds_it_jumps_over` (Review Focus 3): same chart, `advance(events, 0.1)` then `seek(events, 3.0)` then `advance(events, 3.05)` → empty; `advance(events, 3.6)` → one sound at `3.5`.
  - `audio.rs` `a_seek_keeps_a_hold_release_that_is_still_due`: `(120){4}1h[1:1],E`, `seek(events, 1.0)` then `advance(events, 2.1)` → one sound at `2.0` (the release).
  - `audio.rs` `the_metronome_lands_on_the_beat_after_a_seek`: bpm 120, `seek(events, 1.25, 1.0)` → `phase` fraction `0.5`.
  - `spawning.rs` `a_seek_spawns_what_is_on_screen_at_that_time` (Review Focus 1, 2): `(120){4}1h[1:1],2,E`, chart **paused**, push `Seek(1.0)` through `apply_commands` and `next_event` (the `App` harness of the chart-end tests plus `apply_commands`'s resources) → one entity with `HitTime(0.0)` (the hold, mid-hold); the tap at `0.5` is gone; no other note.
  - `spawning.rs` `seeking_twice_leaves_only_the_second_frame` (Review Focus 3): `Seek(1.0)` then `Seek(0.2)` in consecutive updates → the notes are exactly those live at `0.2`.
- [ ] **Step 2: Run** — `cargo test -p engine --lib seek` → FAIL.
- [ ] **Step 3: Implement.** `Restart` stays as it is.
- [ ] **Step 4: Run** — `cargo test -p engine --lib` → PASS; clippy clean; `./scripts/build-wasm.sh`.
- [ ] **Step 5: Commit** — `feat(engine): Seek command (visualizer-majdata 08)`

---

### Task 5: Position and timeline readout (engine → JS)

**Files:**
- Modify: `wasm_bridge.rs`, `systems/mod.rs` (`ingest_songs`, a publish system), `visual/spawning.rs` (`Unload` arm)

**Interfaces:**
- Consumes: `build_timeline`, `Timeline::to_json` (Task 3).
- Produces:
  - Snapshot `PLAYBACK: Mutex<(f64, bool, u32)>` — position (chart seconds), playing, chart id. `pub fn publish_playback(position: f64, playing: bool)`; `pub fn bump_chart(timeline_json: String)` (increments the id and stores the timeline); `pub fn playback_snapshot() -> (f64, bool, u32)` (native, for tests).
  - wasm: `playback_position() -> f64`, `playback_playing() -> bool`, `chart_id() -> u32`, `timeline() -> String` (`""` when no chart).
  - System `publish_playback_state`, every frame, chained after `tick_clock` and `next_event`.
  - `ingest_songs` calls `bump_chart(build_timeline(..).to_json())` after `load_chart`; `Unload` calls `bump_chart(String::new())`.

- [ ] **Step 1: Write the failing test** `wasm_bridge.rs` `a_new_chart_bumps_the_id_and_carries_its_timeline`: `bump_chart("{…}")` twice → id rises by 2 and `timeline()`'s native counterpart returns the second JSON; `publish_playback(1.5, false)` → snapshot `(1.5, false, id)`.
- [ ] **Step 2: Run** — FAIL. **Step 3: Implement.** **Step 4: Run** — `cargo test -p engine --lib` → PASS; clippy clean; `./scripts/build-wasm.sh`.
- [ ] **Step 5: Commit** — `feat(engine): publish playback position and timeline (visualizer-majdata 09)`

---

### Task 6: Host timeline data (frontend, no UI)

**Files:**
- Modify: `apps/host/src/composables/useEngine.ts`
- Create: `apps/host/src/utils/timeline.ts` + `timeline.test.ts`, `apps/host/src/composables/usePlaybackClock.ts` + `usePlaybackClock.test.ts`

**Interfaces:**
- Consumes: Task 4's `seek`, Task 5's readout.
- Produces:
  - `useEngine.ts`: `seek(secs: number): Promise<void>`; `readPlayback(): { position: number; playing: boolean; chartId: number } | null` (synchronous read of the loaded module, `null` before boot); `readTimeline(): Timeline | null` (parses the JSON, `null` when `""`).
  - `utils/timeline.ts`: `type Timeline = { length: number; measures: MeasureCounts[] }`, `type MeasureCounts = { start: number; tap: number; slide: number; touch: number; hold: number }`; `measureAt(tl, t): number` (index of the measure containing `t`, clamped); `measureStart(tl, i): number` (clamped); `timeMarks(length: number, every = 30): number[]` (`30, 60, …` strictly below `length`); `stepMeasure(key: string, current: number, count: number): number | null` (ArrowRight/ArrowUp `+1`, ArrowLeft/ArrowDown `-1`, PageUp `+10`, PageDown `-10`, Home `0`, End `count - 1`, clamped; other keys `null`); `formatClock(secs): string` (`m:ss`).
  - `usePlaybackClock(playback: Playback): { position: Ref<number>; timeline: Ref<Timeline | null>; start(): void; stop(): void }` — one `requestAnimationFrame` loop while started: copies `position`; when the engine's `playing` differs from `playback.playing`, calls `playback.markLoaded(playing)` (the transport shows "play" when the chart ends — Review Focus 5); re-reads the timeline when `chartId` changes.

- [ ] **Step 1: Write the failing tests**:
  - `timeline.test.ts`: `measureAt` at a start, just before the next start, before 0 and past the end; `timeMarks(95)` → `[30, 60, 90]`, `timeMarks(90)` → `[30, 60]`; every `stepMeasure` key at both ends; `formatClock(65.4)` → `"1:05"`.
  - `usePlaybackClock.test.ts` (fake `requestAnimationFrame`, `useEngine` mocked as in `useChartLoader.test.ts`): a frame copies the position; an engine that stopped flips `playback.playing` to false through `markLoaded`; a new `chartId` re-reads the timeline once, an unchanged one does not; `stop()` cancels the frame.
- [ ] **Step 2: Run** — `pnpm test` → FAIL. **Step 3: Implement.** **Step 4: Run** — `pnpm test`, `pnpm build` → PASS.
- [ ] **Step 5: Commit** — `feat(visualizer): timeline data and playback clock (visualizer-majdata 10)`

---

### Task 7: `VizTimeline` (frontend)

**Files:**
- Create: `apps/host/src/components/visualizer/VizTimeline.vue`
- Modify: `apps/host/src/pages/visualizer.vue` (between `VizTransport` and the speed sliders; start/stop `usePlaybackClock` with the page), all nine `locales/*.yaml`, `docs/work/visualizer-majdata/spec.md` (piece 2 status: done)

**Interfaces:**
- Consumes: everything in Task 6.
- Produces: `<VizTimeline :timeline :position :disabled @seek="(secs: number) => void" />`.

Layout, as the spec's reference image: an SVG graph, one bar per measure, height ∝ the measure's total, stacked bottom-up tap / slide / touch / hold in the four colours (CSS custom properties on the component, tuned for both themes); time labels `0:30`, `1:00`, … across the top at `timeMarks`; a vertical playhead at `position`. Under it a slider (`role="slider"`, `aria-valuemin=0`, `aria-valuemax=count-1`, `aria-valuenow` = current measure, `aria-valuetext` "Measure n"), ticks per measure with numbers every 10 (`0, 10, 20 …`). Pointer down / move on either part seeks to the pointer's time (graph) or measure start (slider), at most once per animation frame while dragging; keys through `stepMeasure` seek to `measureStart`. Disabled (no seeking, dimmed) while `timeline` is null.

Strings, all nine locales, under `page.viz`: `timeline` ("Timeline"), `measure` ("Measure {n}").

- [ ] **Step 1: Wire and build** — `pnpm build` passes; `pnpm test` still green.
- [ ] **Step 2: Browser check** — each Review Focus line by hand, on a chart with BPM changes and holds/slides crossing measure lines; drag fast for 10 s and watch the `?debug` frame rate; Escape out of fullscreen with the timeline shown.
- [ ] **Step 3: Commit** — `feat(visualizer): timeline with seeking (visualizer-majdata 11)`
