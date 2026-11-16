# 02 — Known problems of the visualizer on mobile

**What to build:** the visualizer runs on mobile but has known problems
([ADR-0016](../../../adr/0016-visualizer-ungated-on-mobile.md)). This ticket
lists them, ranks them by how many phone users they reach, and fixes them in
that order.

**Blocked by:** None. Issue 01's measurements would help rank the list, but do
not block writing it.

**Status:** done — 2026-11-17, re-measured on the phone after the fixes;
no notice needed

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
- **Status:** **fixed and confirmed** — `Msaa::Off` on the main camera
  (commit `be0d8a3`), after the measurements below isolated multisampling as
  the entire cost. Confirmed on the phone 2026-11-17 with no toggles:
  90.1 fps (the panel's refresh rate), avg 11.1ms, max 15ms, 0 slow, 0
  stalled, same 1138×909 @ 2.81× canvas.

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
- **Status:** **fixed and confirmed** — see the three faults below;
  re-measured 2026-11-17 (*raputa*, note speed 8, Chrome tab): "no note
  missed during the session, the timing is quite good", on the automatic
  lead of 341ms.

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

**Characterised, 2026-11-16:** all three of (a) late, (b) crackle/distortion,
(c) missing in dense sections; both the hit sound and the metronome click.
Output path (speaker / wired / Bluetooth) not yet stated.

**Fault (b) — root cause found and fixed.** Two facts met: both SFX files
peak at exactly **0 dBFS** (`answer.wav`, `clock.wav`, measured), and
bevy_kira_audio 0.25's `with_volume` takes kira's `Decibels`, into which a bare
`f32` converts 1:1. So `with_volume(1.5)` was **+1.5 dB**, not "150%", on a
full-scale sample; kira's renderer hard-clamps the mix at ±1.0
(`backend/renderer.rs`), and every hit clipped. The "duck" `BGM_VOLUME: 0.4`
was likewise +0.4 dB — louder, not quieter. Fixed: every level is now a
negative decibel constant (`GUIDE_SFX_DB` −6, chord ceiling −3, metronome and
BGM −8), a compile-time assertion forbids a level at or above unity, and
`guide_sfx_db` has a test for the chord ceiling. The same clipping existed on
desktop; desktop speakers were just kinder to it.

**Fault (a) — mechanism understood, magnitude unmeasured.** The latency is
structural (two 43ms buffers, main-thread refill, Android output latency) and
cannot be removed from inside the engine; it can be cancelled by asking for
the sound early. Guide sounds now fire from the **chart clock** through
`audio::schedule_guide_sounds` — a cursor over the timed events, replacing the
three `PlayGuideSoundMessage` writes that fired from sprite timers in
`visual/movement/` — and both that scheduler and the metronome run `AudioLead`
milliseconds ahead. The lead defaults to 0 (desktop behaviour unchanged) and
is set from the URL with `?lead=<ms>`; the overlay's last line shows the value
applied. **Measure:** same chart, `?debug&lead=0`, `80`, `120`, `160`, `200`;
report the smallest value at which the tick lands *on* the ring hit. That
number, with the output path it was measured on, decides the mobile default.

**Fault (c) — still present after (a) and (b)** (maintainer, 2026-11-16:
"now it missing hit, but better offset"). One cause found in the code:
`handle_guide_sounds` counted every hit that arrived in a frame and played
**one** sound at chord volume. At 60 FPS two notes 16ms apart — 16th notes
past 230 BPM, or any pseudo-EACH — land in the same frame and became one
sound; at the earlier 23 FPS anything inside 45ms did. In a dense stream that
is exactly "hits go missing". Fixed: the scheduler emits one message per
**hit time** carrying its note count, and the handler plays one sound per
message. A chord is still one louder sound; two distinct times are two
sounds, same frame or not. Regression test
`distinct_hit_times_in_one_frame_are_distinct_sounds`.

What this does *not* fix: two sounds due in the same frame now start on the
same audio buffer, so they are distinct but up to one frame closer together
than written. bevy_kira_audio exposes no delayed start (kira has
`StartTime::Delayed`; the wrapper does not surface it), so sub-frame
placement would mean driving kira directly. Not worth it until (c) is
re-measured with this fix and a specific pattern is named — e.g. "every
second note of a 16th run", versus "random".

**Lead, chosen: option B — derived from the browser.** `AudioContext.baseLatency`
and `outputLatency` are read from the context kira created (captured by the
audio-unlock wrapper), cpal's two 2048-frame buffers are added as 1.5 buffers
at the context's sample rate, and the sum is pushed to the engine whenever it
changes (re-read every 2s: the context reports 0 until a gesture starts it,
and `outputLatency` jumps when Bluetooth connects). The overlay's last line
shows the breakdown; `?lead=<ms>` overrides it. **Validation still owed:** the
number the ear picked in the manual sweep versus what the formula produces,
and on which output path.

**Why the trigger moved to the clock.** With a lead the sound must be asked
for before the sprite reaches the ring, which the sprite's own timer cannot
do. A cursor over `timed_events()` is the metronome's existing pattern, fires
once per hit regardless of frame rate, replays on rewind, and keeps the
headless tracks of a star chain silent (`is_audible`).

**Two sounds the move lost, both restored.** The sprite-timer path fired on
three transitions — entering `Holding` (hold and touch-hold start, touch),
and entering `Dying` for everything but touches and slides — and the first
clock version replayed only the first of them. So (1) a slide's star head,
which reaches the ring as a tap, went silent, and (2) a hold's **release**,
which is a `Dying` transition and not an event in the list, went silent too.
The scheduler now queues each hold's start + duration (`Duration::hold_secs`,
at the BPM in force where the hold starts) when the hold becomes due and
fires it when the clock reaches it, in release order rather than start
order; a release landing on another hit merges into one louder sound; a
rewind drops the pending releases. Nine scheduler tests.

## Done when

- [x] Every problem above is listed with a device, a symptom and a severity
      (device model still "the maintainer's Android phone"; see the last
      comment)
- [x] Problems that block playing a chart are fixed or have a written reason to
      wait — none blocked; P1 and P2 are fixed and confirmed
- [x] Decided whether an "experimental on mobile" notice is still needed (the
      reopen condition in ADR-0016) — not needed

**Re-measured 2026-11-17, Chrome tab on the phone, release build with every
fix above, no URL toggles** (*raputa* — sasakure.UK × TJ.hangneil — note
speed 8, play speed 1.0):

```
90.1 fps  avg 11.1ms  max 15ms
slow >33ms: 0/91  stalled >100ms: 0
canvas 1138x909 @ 2.81x  dpr 2.8125  msaa 1
lead 341ms (auto)  buf 70 + base 87 + out 184 = 341
wasm mem 30MiB  js heap 20MiB
```

Maintainer: "it seems that no note have missed during the session. the
timing is quite good." That answers all three P2 faults and validates the
derived lead on this phone: a manual sweep was not needed, since the
automatic value landed on the hit. The breakdown is itself informative —
`out 184` is Android's output latency and the largest term, `base 87` is
large for a context buffer (a 4096-frame quantum at 44.1kHz, which Chrome
picks on some Android devices), and `buf 70` is cpal's two buffers at
44.1kHz. Output path not stated; the number will move on Bluetooth, and the
2s re-read is what tracks that.

**Notice decision (the last box): not needed.** A chart can be followed end
to end on the phone at the panel's refresh rate with guide sounds on the
hit. ADR-0016's reopen condition — problems that break the visualizer for
most phone users — is not met; the ADR carries a note.

Still unrecorded: the device model and Chrome version (Samsung One UI by
the screenshot). Worth adding to the entry when known, not worth holding
the ticket open for.
