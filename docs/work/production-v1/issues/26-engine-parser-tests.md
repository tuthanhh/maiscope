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
- [ ] Corpus test over the 16 real `maidata.txt` files: parse without panicking,
      snapshot the event counts per difficulty
- [ ] Malformed-input tests: parser returns an error, never panics — a panic in
      wasm takes the whole canvas down
- [ ] Corpus files vendored into `engine/tests/fixtures/` (chart text only, no
      mp3/bg — see issue 01)
- [ ] `cargo test -p engine` wired into CI (issue 15)
