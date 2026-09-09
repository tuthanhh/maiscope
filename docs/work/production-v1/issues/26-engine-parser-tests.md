# 26 — Engine: simai parser test suite

**What to build:** `engine/` currently has **zero** tests — the densest,
most bug-prone, most user-visible code in the repo is entirely unverified.

Parser first, ahead of server tests, because:

- **Highest defect density.** simai is a gnarly text format: mid-chart BPM
  changes, slide syntax, touch notes, `/` and `*` groupings, `h[...]` holds.
- **Zero infrastructure.** Pure functions, native `cargo test -p engine`, no
  Postgres, no browser, no wasm. Milliseconds in CI.
- **Failures are silent.** A bad SQL change 500s loudly into the logs from issue
  06. A misparsed slide renders a *plausible but wrong* chart — nobody reports
  it, and if they do it cannot be reproduced without the exact chart text.
- **It is the phase-2 prerequisite.** Community charts will feed the parser
  syntax nobody has seen. Without a net, every contributed chart is a potential
  visualizer crash.

**Blocked by:** None — start any time, independent of everything else.

**Status:** todo

- [ ] Unit tests per note kind: tap, hold, touch, touch-hold, slide (each shape),
      break/EX variants
- [ ] Timing: BPM change mid-chart, divisor changes, rest handling
- [ ] Re-acquire a chart corpus (the original 16 `maidata.txt` were deleted with
      the audio on 2026-09-09) into the private data repo, chart text only
- [ ] Corpus test: parse every acquired `maidata.txt` without panicking, snapshot
      the event counts per difficulty
- [ ] Fixtures vendored into `engine/tests/fixtures/` — chart text only, never
      mp3 or bg images
- [ ] Malformed-input tests: parser returns an error, never panics — a panic in
      wasm takes the whole canvas down
- [ ] `cargo test -p engine` wired into CI (issue 15)
