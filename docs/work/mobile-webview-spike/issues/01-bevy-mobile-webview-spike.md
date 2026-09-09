# 01 — Spike: release wasm size + Bevy in a mobile WebView

**Type:** prototype / measurement — this answers a question, it does not ship code.

**What to find out:** Whether the Bevy visualizer is viable in an Android WebView.
This is the single unproven assumption in the whole plan, and it is the one most
likely to reverse an earlier decision: if Bevy cannot hold frame rate on a
mid-range phone, the PWA-over-Tauri choice is wrong and an APK bundling the wasm
becomes the only path to v1.1.

Everything else in this plan is known-solvable — Docker, CI, Fly, Neon, caching,
tests. Thousands of people have done all of it. "Does Bevy run acceptably in a
mobile WebView" has no reliable answer to look up.

**Blocked by:** None. Run it early and in parallel — it gates v1.1, not v1.0.

**Status:** todo

- [ ] `./scripts/build-wasm.sh release` run; **raw and brotli'd `.wasm` size
      recorded here** (the dev build is ~74MB; the release number is unknown and
      feeds `web-delivery` issues 04 and 05)
- [ ] `apps/host` served over LAN; opened on a real mid-range Android device
- [ ] One chart loaded end to end (requires the corpus re-acquired in `test-foundation` issue 01, or
      a single `maidata.txt` obtained ad hoc for the spike)
- [ ] Recorded: frame pacing, time to first note, memory ceiling, whether the tab
      is OOM-killed
- [ ] Tested in Android Chrome **and** in an installed PWA — they are not always
      the same engine configuration
- [ ] Outcome recorded as one of:
      - **Runs well** → v1.1 proceeds as planned, flip the issue-25 gate
      - **Renders but janky** → optimise first (`opt-level=z`, `wasm-opt -Oz`,
        sprite atlas trimming), re-measure
      - **Will not run / OOMs** → reopen the Tauri decision; Tauri Android returns
        as the only way to bundle the wasm and get a native surface
