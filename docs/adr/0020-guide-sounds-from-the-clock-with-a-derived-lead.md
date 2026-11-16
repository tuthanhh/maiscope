# ADR-0020 — Guide sounds fire from the chart clock, ahead by a lead derived from the browser

**Status:** accepted
**Date:** 2026-11-17

## Context

On a phone the visualizer's guide sounds were late, clipped and dropped
(`mobile-webview-spike` issue 02, P2). Three separate causes were found:

- **Latency is structural.** kira plays through cpal's `webaudio` backend:
  two 2048-frame buffers refilled from the main thread, then the
  `AudioContext`'s own buffer, then the device's output latency — 184ms on
  the maintainer's phone. None of it can be removed from inside the engine.
- **Clipping was a unit error.** `bevy_kira_audio`'s `with_volume` takes
  decibels; the levels were written as gains (`1.5`), so every hit was
  +1.5 dB on a sample that already peaked at 0 dBFS, through a renderer that
  hard-clamps.
- **Drops were coalescing.** Sounds were requested from sprite timers and
  the handler played one sound per frame; two hits in one frame became one.

The sprite-timer trigger could not be kept: a sound that must start before
the sprite reaches the ring cannot be requested by the sprite arriving.

## Decision

1. **Trigger from the chart clock.** `audio::schedule_guide_sounds` walks
   the timed events with a cursor, like the metronome, and emits one message
   per *hit time* carrying its note count; the handler plays one sound per
   message. Hold and touch-hold releases are queued as start + duration.
   Slide star heads sound; the headless tracks of a star chain do not.
2. **Run that cursor ahead by a lead**, `AudioLead` in wall milliseconds
   converted to chart seconds at the current play speed, applied to the
   metronome too.
3. **Derive the lead from the browser**, not from a per-device table:
   `AudioContext.baseLatency + outputLatency` as reported by the context kira
   created (captured by the audio-unlock wrapper), plus 1.5 cpal buffers at
   the context's sample rate. Re-read every two seconds, because the context
   reports zero until a gesture starts it and `outputLatency` changes when
   Bluetooth connects. `?lead=<ms>` overrides it for measurement.
4. **Levels are negative decibels**, with a compile-time assertion that no
   level reaches unity.

Measured on the maintainer's phone: derived lead 341ms (70 + 87 + 184),
sounds on the hit, no drops, without a manual sweep.

## Rejected alternatives

**A manual lead per device**, chosen by ear and stored. Works for one phone;
wrong the moment headphones or Bluetooth change the output path, and asks
every user to tune something.

**Keep the sprite-timer trigger and delay the visuals instead.** Shifts the
whole chart late to meet the audio; the ring hit is the thing the viewer
reads, so it is the wrong side to move.

**Drive kira directly for sub-frame placement** (`StartTime::Delayed`,
which `bevy_kira_audio` does not expose). Two sounds due in the same frame
still start on the same audio buffer, up to one frame closer together than
written. Not taken: at 90 fps the error is 11ms and nobody heard it. Open
if a chart ever makes it audible.

## Consequences

Desktop behaviour is unchanged in principle (the lead was always paid,
unseen) and improved in fact (the clipping existed there too). The engine
gained a read-only outbox (`frame_stats`) and three runtime knobs on the
command queue (`set_msaa`, `set_render_scale`, `set_audio_lead_ms`) that the
`?debug` overlay reads and writes; they stay, since the next device will
need them.
