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

Then one hypothesis at a time, smallest change first: `Msaa::Off` on the
camera is a one-line test of candidate 1 and the most likely win.

## Done when

- [ ] Every problem above is listed with a device, a symptom and a severity
- [ ] Problems that block playing a chart are fixed or have a written reason to
      wait
- [ ] Decided whether an "experimental on mobile" notice is still needed (the
      reopen condition in ADR-0016)
