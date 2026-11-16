# 02 — Known problems of the visualizer on mobile

**What to build:** the visualizer runs on mobile but has known problems
([ADR-0016](../../../adr/0016-visualizer-ungated-on-mobile.md)). This ticket
lists them, ranks them by how many phone users they reach, and fixes them in
that order.

**Blocked by:** None. Issue 01's measurements would help rank the list, but do
not block writing it.

**Status:** in-progress — one problem reported and instrumented, not yet
measured (2026-11-16)

## Problems

> Each entry needs the device and browser it was seen on, what happens, and
> whether it blocks playing a chart or only degrades it.

### P1 — Jerky frames and stuttering guide sounds

- **Seen on:** installed PWA on the maintainer's phone (Android, model and
  Chrome version to be recorded), release wasm from `maiscope.pages.dev`,
  2026-11-16. The same chart is smooth on desktop.
- **Symptom:** notes, slides and touches all render and arrive **on time**,
  but motion is jerky and the guide sounds stutter — "especially the sound".
- **Severity:** degrades. A chart can be followed; it cannot be enjoyed.
- **Status:** cause not yet established. Instrumented, see below.

**What is known from the code, before measuring.** "On time" rules out the
chart clock: it advances on Bevy's virtual delta
(`engine/src/systems/mod.rs`, `tick_clock`), which Bevy clamps at 250ms per
frame (`Time<Virtual>::DEFAULT_MAX_DELTA`), so a phone dropping below 4 FPS
would run the chart *slow*, and it does not. That leaves frame pacing: frames
that are individually late, often enough to see. The sound is the same
problem heard rather than seen: kira's web backend (cpal 0.15 `webaudio`)
schedules every audio buffer with a `setTimeout` on the main thread
(`cpal/src/host/webaudio/mod.rs`, `set_timeout_with_callback`), so a frame
that overruns delays the next buffer, and a late buffer is a click or a
dropped guide tick. No separate audio bug to chase until frame pacing is
fixed.

Candidates, none confirmed:

1. **Fill rate.** Nothing sets MSAA, so the camera runs at Bevy's default
   4 samples; `fit_canvas_to_parent` plus a 2–3× `devicePixelRatio` makes the
   backing canvas several million multisampled pixels full of alpha-blended
   sprites. The classic mobile GPU wall.
2. **Per-frame tessellation.** Touch-hold countdowns rebuild a lyon path every
   frame per edge (`visual/movement/animation.rs`, `ShapeBuilder` in the
   countdown loop). CPU work scaled by live touch holds.
3. **Entity count.** One chevron sprite per 30 world units of slide; long or
   fanned slides are dozens of entities with per-frame transforms.

**Instrumentation added so a phone can report numbers.** The engine now
publishes a one-second frame-pacing snapshot
(`engine/src/systems/frame_stats.rs`) through `wasm_bridge::frame_stats()`,
and the visualizer page overlays it on the canvas when the debug flag is on:
fps, average and max frame time, how many frames exceeded 33ms (visibly late)
and 100ms (long enough to starve audio), the physical canvas size and scale
factor. The flag is `?debug` on the visualizer route
(`#/visualizer?songId=…&debug`) and it **sticks in `localStorage`**, because
an installed PWA has no address bar: open `?debug` once in a Chrome tab on
the phone, then launch the installed app, and the overlay is there.
`?debug=0` clears it. The first measurement to take is that overlay on the
phone, for a busy chart, written into this entry — noting whether it was read
in the Chrome tab or the installed app, since ticket 01 expects those to
differ.

**First measurement, 2026-11-16 — Chrome tab on the phone** (the maintainer
reports the installed PWA behaves the same; model and Chrome version still to
be recorded), release wasm from `maiscope.pages.dev`, during a busy chart:

```
23.3 fps  avg ~45ms  max 55–70ms
slow >33ms: 19/25  stalled >100ms: 0
canvas 1138x909 @ 2.81x  dpr 2.8125
```

What it says: **uniformly slow, not hitchy.** Three of every four frames are
late and none stalls, so this is steady per-frame cost, not garbage-collection
pauses or asset loads. The chart stays on time because 70ms never reaches the
250ms clamp. The audio stutter follows directly: at 45ms per frame the
main-thread `setTimeout` that feeds the next audio buffer is routinely late.
The canvas is ~1.0 Mpx physical (a ~405×323 CSS box at 2.8×), multisampled 4×
by default — the fill-rate suspect is live but not yet proven; 45ms could as
easily be CPU-bound ECS in single-threaded wasm.

**Next: one variable at a time, without a deploy per guess.** The engine
gained two runtime knobs on the same command queue as transport —
`set_msaa(samples)` and `set_render_scale(scale)` — and the visualizer reads
them from the URL: `?msaa=1` (off; `4` is the default) and `?scale=1` (render
at CSS pixels, ignoring devicePixelRatio; `0` restores). The overlay's third
line shows the applied values. Measure, in this order, same chart, same
section, ~10s each:

| URL | Tests |
|---|---|
| `?debug` | baseline (already: 23 fps) |
| `?debug&msaa=1` | MSAA cost alone |
| `?debug&scale=1` | fill rate alone (canvas drops to ~405×323) |
| `?debug&msaa=1&scale=1` | both — the GPU floor |

If the last row is still ~23 fps, the GPU is not the bottleneck and candidates
2 and 3 (CPU) are next. If `msaa=1` alone recovers most of it, the fix is one
line on the camera.

**Second measurement, 2026-11-16 — `?debug&msaa=1&scale=1`:** avg 16.7ms, max
21ms, slow 0, stalled 0 — a locked 60 FPS. **The GPU owned the frame.**
Candidates 2 and 3 are off the table for the frame rate. Still needed before
choosing the fix: `msaa=1` *alone* and `scale=1` *alone*, because the two
remedies cost different things — MSAA off costs nothing visible on textured
sprites (only lyon shapes alias), while rendering at CSS pixels on a 2.8×
screen is visibly soft.

**And the sound is still bad at 60 FPS**, so the audio stutter is *not* the
frame-pacing symptom it was assumed to be above. Separate problem, separate
investigation — see P2.

### P2 — Guide sounds still wrong at a steady 60 FPS

- **Seen on:** same phone, Chrome tab, after the GPU toggles (P1); frame
  readout clean.
- **Symptom:** reported as "the sound still bad". Not yet characterised —
  late, crackling, missing, or stuttering are four different faults.
- **Severity:** degrades.
- **Status:** investigating; see below.

**What the code says.** kira on wasm plays through cpal's `webaudio` backend
(`cpal/src/host/webaudio/mod.rs`): two `AudioBuffer`s are alternated, each
refilled on the main thread by the previous one's `onended` callback (the
first pair kicked off by `setTimeout`). Every refill runs kira's mixer
synchronously inside that callback. The timing margin is one buffer; a
refill that lands late is a gap. That mechanism is also inherently high-latency
— a sound starts at the earliest one buffer and a scheduling hop after it is
requested, on top of Android's own output latency, which is large. On desktop
both are small enough to go unnoticed.

Also on the table: `GUIDE_SFX_VOLUME` is 2.0, capped at 1.5 — a gain above
unity on a short transient, through a phone speaker, is a plausible crackle on
its own; and Bluetooth output adds 100–300ms that no engine change can remove.

## Done when

- [ ] Every problem above is listed with a device, a symptom and a severity
- [ ] Problems that block playing a chart are fixed or have a written reason to
      wait
- [ ] Decided whether an "experimental on mobile" notice is still needed (the
      reopen condition in ADR-0016)
