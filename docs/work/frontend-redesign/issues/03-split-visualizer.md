# 03 — Split `pages/visualizer.vue`

**What to build:** break up the 1047-line page — three times the next-largest
file in the app.

It currently holds the engine bridge, chart fetching, manual paste, and playback
controls in one component.

**Blocked by:** None. Independent of the API work, so it can land at any time.

**Status:** todo

- [ ] Engine lifecycle stays in `useEngine.ts`; the page stops reaching past it
- [ ] Chart loading separated from playback controls
- [ ] Manual paste — the fallback for a sheet with no chart — becomes its own
      component
- [ ] No behaviour change. This is legibility, not rework
- [ ] The engine boundary is untouched: JS↔Bevy still goes through
      `wasm_bridge.rs`'s mailbox, drained once per frame by `apply_commands`,
      never called synchronously
