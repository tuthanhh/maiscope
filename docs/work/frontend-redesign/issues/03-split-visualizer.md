# 03 — Split `pages/visualizer.vue`

**What to build:** break up the 1047-line page (1208 by the time it was split) — three times the next-largest
file in the app.

It currently holds the engine bridge, chart fetching, manual paste, and playback
controls in one component.

**Blocked by:** None. Independent of the API work, so it can land at any time.

**Status:** done

- [x] Engine lifecycle stays in `useEngine.ts`; the page stops reaching past it
- [x] Chart loading separated from playback controls
- [x] Manual paste — the fallback for a sheet with no chart — becomes its own
      component
- [x] No behaviour change. This is legibility, not rework
- [x] The engine boundary is untouched: JS↔Bevy still goes through
      `wasm_bridge.rs`'s mailbox, drained once per frame by `apply_commands`,
      never called synchronously

## Comments

**2026-11-17 — built.** The page is 204 lines of wiring. Four composables
hold the logic: `useAutoLead` (the two-second audio-lead refresh and
`?lead=`), `useDebugOverlay` (the sticky `?debug` flag and frame-stats
lines), `usePlayback` (transport state, speeds, play/restart), and
`useChartLoader` (deep link and paste, sheet metadata), which drives the
transport through `usePlayback` rather than owning it. Five components hold
the markup, each with its styles cut verbatim from the old page:
`VizStage` (canvas host, boot overlay, debug readout; exposes `host` for the
page to mount the engine canvas into), `VizTransport`, `VizSlider` (used for
both speeds), `VizSheetCard`, `VizPastePanel`.

Checked in headless Chromium against the local API: a deep link renders
pixel-identically to the unsplit page apart from the live frame stats; the
steppers, the play-speed slider, play/pause and loading pasted text behave
as before, with no console errors. The engine boundary is untouched — every
engine call still goes through `useEngine.ts`.
