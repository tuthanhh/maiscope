# Visualizer piece 1 — player behaviour: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Play opens empty, pause freezes every on-screen element at once, and the stage can go fullscreen.

**Architecture:** Pause is fixed where it breaks — the note-lifecycle system (`update_movement`) advances per-note `Timer`s by wall time even while the chart clock is paused, so it now advances them by a delta that is zero while paused. Leaving Play sends a new `Unload` mailbox command that despawns notes, empties the chart and drops the music; the page stops replaying an old chart on mount. Fullscreen is a page-level toggle over the Fullscreen API on the stage element.

**Tech Stack:** Rust (Bevy 0.18, bevy_kira_audio 0.25, wasm-bindgen `=0.2.122`), Vue 3 + TypeScript, Vitest.

**Spec:** [`docs/work/visualizer-majdata/spec.md`](spec.md), piece 1. **Planning change, recorded in the spec:** "every on-screen element is a function of chart time" moves to piece 2 — seeking needs it; pausing does not. Piece 1 gates the existing timers on the clock instead.

## Global Constraints

- Never `cargo build --workspace` / `cargo test --workspace`; engine: `cargo test -p engine` (needs `libasound2-dev`, `libudev-dev`); wasm: `./scripts/build-wasm.sh`.
- `wasm-bindgen` stays pinned at `=0.2.122` (engine `Cargo.toml` and `scripts/build-wasm.sh`).
- JS reaches Bevy only through the mailbox in `engine/src/wasm_bridge.rs`, drained once per frame by `apply_commands` — no synchronous calls into Bevy.
- Every new UI string goes into all nine `apps/host/src/locales/*.yaml` files.
- `cargo fmt --check`, `cargo clippy -p engine --target wasm32-unknown-unknown -- -D warnings`, `pnpm test`, `pnpm build` clean before each commit.
- One commit per task, on branch `v2`, message ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Pause then resume with music loaded** — notes resume exactly where they froze and stay locked to the BGM (the clock follows the audio position; the timers must not have run ahead).
2. **Pause in the middle of a hold or a slide trace** — the hold body and the slide star stop mid-way, not at the end.
3. **Leaving Play while a deep-linked chart is still downloading** — the chart that arrives after `Unload` must not start playing in the background or reappear on the next visit.
4. **Fullscreen left with Escape** (not the toggle) — the toggle's state and icon follow, and the layout returns.
5. **A deep link after leaving and coming back** — `?song=&type=&difficulty=` still loads and plays on the second visit.

Each line's test lives in the task that owns it (marked "Review Focus n").

---

### Task 1: Pause freezes every note (engine)

**Files:**
- Modify: `engine/src/systems/chart_playback.rs` (add method; add `#[cfg(test)] mod tests`)
- Modify: `engine/src/systems/visual/movement/mod.rs` (every `timer.tick(time.delta())` in `update_movement`, lines ~214–437)

**Interfaces:**
- Produces: `ChartPlayback::visual_delta(&self, wall: std::time::Duration) -> std::time::Duration` — `wall` while playing, `Duration::ZERO` while paused.

- [ ] **Step 1: Write the failing tests** in `chart_playback.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn visual_time_stops_while_paused() {
        let mut chart = ChartPlayback::default();
        chart.pause();
        assert_eq!(chart.visual_delta(Duration::from_millis(16)), Duration::ZERO);
    }

    #[test]
    fn visual_time_runs_at_wall_speed_while_playing() {
        // Phase durations are already divided by chart_speed when each phase
        // starts, so the timers tick in wall time, not chart time.
        let mut chart = ChartPlayback::default();
        chart.set_chart_speed(2.0);
        chart.resume();
        assert_eq!(chart.visual_delta(Duration::from_millis(16)), Duration::from_millis(16));
    }
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p engine chart_playback`
Expected: FAIL — `no method named visual_delta`.

- [ ] **Step 3: Implement `visual_delta`**, then in `update_movement` compute `let dt = chart.visual_delta(time.delta());` once and replace every `timer.tick(time.delta())` with `timer.tick(dt)` (Growing, Moving, Holding, Waiting, Sliding, Dying). Effects spawned as their own entities (the slide-head pop, touch sparks) ride the same `NoteTiming` and so freeze too. Leave `frame_stats.rs` on `Time` (it measures frames, not the chart).

- [ ] **Step 4: Run the tests and the engine checks**

Run: `cargo test -p engine && cargo clippy -p engine --target wasm32-unknown-unknown -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Browser check (Review Focus 1, 2)** — `./scripts/build-wasm.sh`, `pnpm dev`, open a chart with audio via a deep link: pause during a hold and during a slide trace; every element stops; resume; notes stay on the beat for 30 s.

- [ ] **Step 6: Commit** — `fix(engine): pause freezes every note (visualizer-majdata 01)`

---

### Task 2: `Unload` command (engine)

**Files:**
- Modify: `engine/src/systems/chart_playback.rs` (method + test)
- Modify: `engine/src/wasm_bridge.rs` (`EngineCommand::Unload`, `#[wasm_bindgen] pub fn unload()`)
- Modify: `engine/src/systems/visual/spawning.rs` (`apply_commands` arm)

**Interfaces:**
- Produces (Rust): `ChartPlayback::unload(&mut self)` — empties `timed_events`, `next_spawn_index = 0`, `elapsed_time = 0.0`, `is_playing = false`; `chart_speed` and `note_speed` are kept (they are the user's settings).
- Produces (wasm): `unload()` → pushes `EngineCommand::Unload`.

- [ ] **Step 1: Write the failing test** in `chart_playback.rs`'s test module:

```rust
#[test]
fn unload_empties_the_chart_and_keeps_the_user_speeds() {
    let mut chart = ChartPlayback::default();
    chart.load_chart(simai::parse_chart("(120){4}1,2,3,4,E").unwrap());
    chart.set_chart_speed(1.5);
    chart.set_note_speed(8.0);
    chart.tick(1.0);
    chart.unload();
    assert!(chart.timed_events().is_empty());
    assert_eq!(chart.elapsed_time(), 0.0);
    assert!(!chart.is_playing());
    assert_eq!(chart.chart_speed(), 1.5);
    assert_eq!(chart.note_speed(), 8.0);
}
```

- [ ] **Step 2: Run to see it fail** — `cargo test -p engine unload` → FAIL, `no method named unload`.

- [ ] **Step 3: Implement** `ChartPlayback::unload`; `EngineCommand::Unload` and `unload()` in `wasm_bridge.rs` beside `restart()`; the `apply_commands` arm despawns every `NoteTiming` entity (as `Restart` does), calls `chart.unload()`, `bgm_channel.stop()`, `commands.remove_resource::<BgmInstance>()` and `commands.remove_resource::<BgmSource>()`, and resets the guide scheduler (`ResMut<audio::GuideScheduler>::reset()`, as `ingest_songs` does).

- [ ] **Step 4: Run** — `cargo test -p engine && cargo clippy -p engine --target wasm32-unknown-unknown -- -D warnings` → PASS.

- [ ] **Step 5: Commit** — `feat(engine): Unload command (visualizer-majdata 02)`

---

### Task 3: Play opens empty (host)

**Files:**
- Modify: `apps/host/src/composables/useEngine.ts` (add `unloadChart`; remove `hasChart` and `chartLoaded`)
- Modify: `apps/host/src/pages/visualizer.vue` (`onMounted` replay branch, `onUnmounted`)
- Modify: `apps/host/src/composables/useChartLoader.ts` (ignore loads that land after leaving)
- Test: `apps/host/src/composables/useChartLoader.test.ts`

**Interfaces:**
- Consumes: wasm `unload()` (Task 2).
- Produces: `unloadChart(): Promise<void>` in `useEngine.ts`; `useChartLoader(...)` gains `dispose(): void` — after it, `autoLoad` and an in-flight fetch never call `loadChart`/`loadSong`.

- [ ] **Step 1: Write the failing test** (Review Focus 3) — mock `~/composables/useEngine` (`fetchChart` returns a promise you resolve by hand, `fetchSheetInfo` resolves, `loadChart` is a spy):

```ts
it("drops a chart that arrives after the page has gone", async () => {
  const loader = useChartLoader(fakePlayback());
  const pending = loader.autoLoad({ song: "s", type: "dx", difficulty: "master" });
  loader.dispose();
  resolveChart("(120){4}1,E");
  await pending;
  expect(loadChart).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Run** — `pnpm test useChartLoader` → FAIL (`dispose` is not a function).

- [ ] **Step 3: Implement** `dispose()` (a `disposed` flag checked after every await before touching the engine); `unloadChart()` (calls the module's `unload()`); in `visualizer.vue`, `onUnmounted` calls `loader.dispose()` then `unloadChart()` instead of `pause()`, and the `hasChart() && !hasDeepLink(...)` replay branch in `onMounted` is deleted. The page's `simai` and `sheetInfo` already reset with the page.

- [ ] **Step 4: Run** — `pnpm test && pnpm build` → PASS.

- [ ] **Step 5: Browser check (Review Focus 3, 5)** — open a deep link, leave to Songs before it finishes loading, come back to Play with no link: empty playfield, silent. Open the deep link again: it loads and plays.

- [ ] **Step 6: Commit** — `feat(visualizer): Play opens empty (visualizer-majdata 03)`

---

### Task 4: Fullscreen toggle (host)

**Files:**
- Create: `apps/host/src/composables/useFullscreen.ts`
- Test: `apps/host/src/composables/useFullscreen.test.ts`
- Modify: `apps/host/src/components/visualizer/VizTransport.vue` (a toggle button), `apps/host/src/pages/visualizer.vue` (pass the stage element; `:fullscreen` styles), `apps/host/src/components/ui/icons.ts` (Lucide `maximize`, `minimize`, from lucide-static 0.460.0 like the rest), `apps/host/src/locales/*.yaml` (`page.viz.fullscreen`, `page.viz.exitFullscreen`)

**Interfaces:**
- Produces: `useFullscreen(target: Ref<HTMLElement | null>): { supported: boolean; active: Ref<boolean>; toggle(): Promise<void> }` — `supported` is `document.fullscreenEnabled`; `active` follows `fullscreenchange` (so Escape updates it); listener removed on unmount.

- [ ] **Step 1: Write the failing tests** (Review Focus 4) — stub `document.fullscreenEnabled`, `element.requestFullscreen`, `document.exitFullscreen`, `document.fullscreenElement`:

```ts
it("enters on toggle and follows an exit made with Escape", async () => {
  const el = ref(document.createElement("div"));
  const fs = useFullscreen(el);
  await fs.toggle();
  expect(el.value!.requestFullscreen).toHaveBeenCalled();
  setFullscreenElement(el.value);  document.dispatchEvent(new Event("fullscreenchange"));
  expect(fs.active.value).toBe(true);
  setFullscreenElement(null);      document.dispatchEvent(new Event("fullscreenchange"));
  expect(fs.active.value).toBe(false);
});

it("is unsupported where the API is missing (iPhone Safari)", () => {
  stubFullscreenEnabled(false);
  expect(useFullscreen(ref(null)).supported).toBe(false);
});
```

- [ ] **Step 2: Run** — `pnpm test useFullscreen` → FAIL (module not found).

- [ ] **Step 3: Implement** `useFullscreen`; the transport shows an icon button (`maximize` / `minimize`, labelled with the two new strings) only when `supported`; the target is the stage element (canvas, transport, sliders). In fullscreen the stage is centred on `--bg` with the canvas as large as the viewport allows (`.mv-viz-stage:fullscreen`).

- [ ] **Step 4: Run** — `pnpm test && pnpm build` → PASS; locale key check (every `t("…")` key in all nine files) clean.

- [ ] **Step 5: Browser check** — desktop Chromium: toggle in, Escape out, toggle in and out; the engine keeps rendering at the new size.

- [ ] **Step 6: Commit** — `feat(visualizer): fullscreen toggle (visualizer-majdata 04)`, with the spec's piece-1 status set to done and its planning change recorded.
