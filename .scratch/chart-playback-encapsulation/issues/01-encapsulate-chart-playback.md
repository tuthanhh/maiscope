# 01 — Encapsulate ChartPlayback behind named methods

**What to build:** `engine/src/systems/chart_playback.rs`'s `ChartPlayback` resource stops exposing five public fields that five call sites across four files mutate directly. All fields become private; a method surface (`load_chart`, `restart`, `pause`, `resume`, `tick`, `sync_to_audio_position`, `set_chart_speed`, `set_note_speed`, plus getters) replaces every raw read/write, so the invariant "these fields always change together" (chart-load reset, restart) becomes structurally true instead of true-by-convention. Behavior must be bit-identical to today — this is a pure encapsulation refactor, not a behavior change.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `chart_playback.rs` rewritten: five fields private, full method surface added (`load_chart`, `restart`, `pause`, `resume`, `tick`, `sync_to_audio_position`, `set_chart_speed`, `set_note_speed`, `chart_speed()`, `note_speed()`, `is_playing()`, `has_events()`, `is_finished()`, `timed_events()`, `elapsed_time()`); `advance()`/`approach_time()` unchanged
- [ ] `systems/mod.rs` (`check_if_ready`, `tick_clock`, `ingest_songs`) migrated to the new methods
- [ ] `systems/visual/spawning.rs` (`next_event`, `apply_commands`) migrated
- [ ] `systems/audio.rs` (`tick_metronome`, `start_bgm`) migrated
- [ ] `systems/visual/movement/mod.rs` movement-speed system migrated
- [ ] `cargo build -p maiscope-viewer` compiles clean, zero warnings about fields needing `#[allow(dead_code)]`
- [ ] Manual smoke test via `./engine/build-wasm.sh` + `pnpm dev`: notes spawn/scroll in sync, pause/resume, note/song speed sliders, restart, chart-completion stop, metronome-through-restart — all match pre-refactor behavior exactly
