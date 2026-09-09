# 25 — Gate the visualizer off mobile for v1.0

**What to build:** v1.0 ships browse-only. The visualizer stays reachable on
desktop browsers and is gated behind a clear message on mobile until the spike
(issue 28) says Bevy is viable in a mobile WebView.

This is not a failure state — the catalog browser is the entire upstream
arcade-songs product and is useful alone. What it avoids is shipping a
white-screen or an OOM-killed tab as the headline feature.

**Blocked by:** 21

**Status:** todo

- [ ] Detection chosen and documented: viewport/pointer capability rather than
      user-agent sniffing
- [ ] Gated route shows an honest message ("desktop only for now"), not a dead
      link or a spinner
- [ ] The wasm chunk is **not** loaded on gated devices — the dynamic import in
      `composables/useEngine.ts` must never fire, or mobile users pay ~20MB for a
      page they cannot use
- [ ] "Visualize" affordances hidden or disabled on gated devices, including the
      per-sheet `hasChart` entry point
- [ ] Override query param for testing on a real phone during issue 28
- [ ] Gate is a single flag, easy to flip for v1.1
