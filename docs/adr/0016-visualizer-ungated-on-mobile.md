# ADR-0016 — The visualizer ships ungated on mobile

**Status:** accepted
**Date:** 2026-11-04
**Amends:** the v1.0 scope in [`ROADMAP.md`](../ROADMAP.md) ("gated off mobile")
and [`web-delivery`](../work/web-delivery/spec.md) issue 05

## Context

The v1.0 plan gated the visualizer off mobile browsers. The reason was risk, not
evidence: Bevy in a mobile WebView was unproven, and a white screen or an
OOM-killed tab as the headline feature was judged worse than shipping the catalog
browser alone. `web-delivery` issue 05 specified the gate, and the
[`mobile-webview-spike`](../work/mobile-webview-spike/issues/01-bevy-mobile-webview-spike.md)
was to decide when to lift it in v1.1.

Issue 05 was never built. The visualizer is live and reachable on phones today,
and on 2026-11-04 the maintainer reported that it **works on mobile**, with a
number of problems still to fix. That report is an informal observation, not the
spike's measurements: frame pacing, time to first note, memory ceiling and the
installed-PWA case are still unrecorded.

The risk the gate guarded against — the tab not running at all — did not happen.

## Decision

v1.0 ships the visualizer ungated on mobile. `web-delivery` issue 05 is dropped.
The known mobile problems are tracked as a v1.1 ticket, and v1.1 becomes "mobile
visualizer polish" rather than "mobile visualizer enablement".

## Rejected alternatives

**Build the gate anyway, as specified.** For: it keeps mobile visitors from
paying the wasm download (~17MB release) and the sprite runtime cache for a page
with known problems, and it makes the v1.0 headline "works everywhere it is
offered". Against: it hides a feature that runs, takes a ticket of work
(detection, an honest message, keeping the dynamic import from firing, an override
param) to remove capability, and would be deleted again in v1.1. Not chosen
because the failure it protects against was not observed.

**Ship ungated with an "experimental on mobile" notice.** For: it sets
expectations at almost no cost, and is the honest middle ground while the
problems stand. Against: it is still a ticket, and the problems are not yet
itemised, so the notice cannot say what to expect. Not chosen for now; it remains
the cheapest fix if the problems turn out to be visible to most phone users.

## Consequences

**Positive:** v1.0 loses a `todo` ticket, mobile users get the visualizer now,
and v1.1 work starts from a running baseline instead of a gate flip.

**Negative:**

- A phone that opens the visualizer downloads the release wasm (~17MB) and the
  sprite set, and then meets the known problems. Nothing tells the visitor the
  page is rough on mobile.
- The known problems are not yet written down in detail. Until they are, nobody
  can judge which ones block real use. Owner: the maintainer, in
  [`mobile-webview-spike` issue 02](../work/mobile-webview-spike/issues/02-mobile-visualizer-known-problems.md).
- The spike's measurements are still missing, so "works on mobile" is one
  observation on unknown devices, not a frame-rate or memory number.

**Neutral:** [ADR-0003](0003-web-pwa-drop-tauri.md) is not reopened. Its trigger
is Bevy failing to hold frame rate in an Android WebView, which this observation
does not show — but the spike that would confirm it has not been run either.

**This decision reopens if** the problems in issue 02 turn out to break the
visualizer for most phone users. The fallback is the notice above, not the full
gate.

**2026-11-17: measured, not reopened.** Issue 02's problems were two — a
GPU fill-rate cost (MSAA, fixed) and guide-sound latency, clipping and
coalescing (fixed, [ADR-0020](0020-guide-sounds-from-the-clock-with-a-derived-lead.md)).
After the fixes the phone holds its panel refresh rate with 30MiB of wasm
memory and sounds on the hit. No notice is shown.
