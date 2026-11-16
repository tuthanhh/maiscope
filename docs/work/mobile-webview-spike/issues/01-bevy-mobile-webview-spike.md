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

**Status:** in-progress — instrumented, measurements owed (2026-11-16)

- [x] `./scripts/build-wasm.sh release` run; **raw and brotli'd `.wasm` size
      recorded here** — 2026-11-16, `wasm-release` profile (opt-level z, fat
      LTO, strip, no `wasm-opt`): **12.3MB raw, 2.6MB Brotli** (`-q 11`).
      The dev build on the same day was 140MB. (The ~74MB dev figure below
      predates Bevy 0.18.)
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

## Comments

**Most of this spike has already been run, by accident, while fixing the
visualizer on the maintainer's phone** ([issue 02](02-mobile-visualizer-known-problems.md),
2026-11-16): a chart loaded end to end over `maiscope.pages.dev`, frame
pacing was measured in a Chrome tab (23 fps, then 60 fps after `Msaa::Off`),
and the installed PWA was reported to behave the same. What that session did
not record is what this ticket is for: device model and Chrome version, the
release wasm size, memory, time to first note, and whether the tab is ever
killed. The outcome box can only be "runs well" or "renders but janky" now —
the "will not run" branch is already closed.

**How to read memory on the phone.** The `?debug` overlay (issue 02) gained a
last line: `wasm mem NMiB` is the wasm instance's linear memory, kept from
`init()` in `useEngine.ts`. `WebAssembly.Memory` only grows, so this is a
high-water mark — exactly the "memory ceiling" asked for — not a live figure.
`js heap NMiB` follows on Chrome (`performance.memory`, non-standard; absent
elsewhere). Read both at the end of the busiest chart available, in the tab
and in the installed app. An OOM shows up as the tab silently reloading, not
as an error.
