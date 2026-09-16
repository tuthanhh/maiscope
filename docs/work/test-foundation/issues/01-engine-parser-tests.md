# 01 — Engine: simai parser test suite

**What to build:** `engine/` currently has **zero** tests — the densest,
most bug-prone, most user-visible code in the repo is entirely unverified.

Parser first, ahead of server tests, because:

- **Highest defect density.** simai is a gnarly text format: mid-chart BPM
  changes, slide syntax, touch notes, `/` and `*` groupings, `h[...]` holds.
- **Zero infrastructure.** Pure functions, native `cargo test -p engine`,
  no Postgres, no browser, no wasm. Milliseconds in CI.
- **Failures are silent.** A bad SQL change 500s loudly into the logs from issue
  06. A misparsed slide renders a *plausible but wrong* chart — nobody reports
  it, and if they do it cannot be reproduced without the exact chart text.
- **It is the phase-2 prerequisite.** Community charts will feed the parser
  syntax nobody has seen. Without a net, every contributed chart is a potential
  visualizer crash.

**Blocked by:** None — start any time, independent of everything else.

**Status:** done

- [x] Unit tests per note kind: tap, hold, touch, touch-hold, slide (each shape),
      break/EX variants
- [x] Timing: BPM change mid-chart, divisor changes, rest handling (`chart.rs`)
- [x] Corpus test: parse every fixture without panicking, snapshot the event
      counts (`engine/tests/corpus.rs`)
- [x] Fixtures under `engine/tests/fixtures/` — chart text only, never mp3 or bg
      images
- [x] Malformed-input tests: parser returns an error, never panics — a panic in
      wasm takes the whole canvas down (one rejection test per module, plus
      `chart::tests::parse_chart_never_panics` over 16 hostile inputs)
- [x] `cargo test -p engine` wired into CI as its own `engine` job

## Decisions taken while scaffolding

The suite was built in two passes: a scaffold of 5 worked examples plus 35
`#[ignore = "TODO"]` stubs, each carrying a doc comment naming what to assert,
then the assertions themselves. The stubs are all filled — see Result below.

- **Inline `#[cfg(test)]` modules, not `engine/tests/`.** `parse_note`,
  `parse_slide_note` and `parse_duration_bracket` are private to
  `systems/parser/`; only `parse_chart` is public. An integration test links the
  crate as an external consumer and cannot see the rest.
- **`PartialEq` derived** on `ChartEvent`, `Note`, `NoteKind`, `SlideSegment`
  and `SlideShape` so tests are plain `assert_eq!` against a literal expected
  value, with a full `Debug` diff on failure.
- **`pub mod chart` façade in `lib.rs`** re-exporting `parse_chart` and the
  event types, so `tests/corpus.rs` can reach them without making `systems`
  public.
- **Synthetic fixtures**, hand-written for this repo, over vendored upstream
  `maidata.txt` — committable without a licensing question. Real charts stay in
  the gitignored `songs/`.
- **Its own CI job** with cache key `engine-native` and an apt install of
  `libasound2-dev` / `libudev-dev`, rather than growing the `rust` job's cache
  by Bevy's dependency tree on every server PR.

## Result

58 tests, none ignored: `duration.rs` 8, `note.rs` 15, `chart.rs` 17,
`slide.rs` 17, plus 2 corpus tests. `cargo clippy -p engine
--all-targets` is clean.

Assertions are checked against
[`docs/reference/simai-notation.md`](../../../reference/simai-notation.md) rather
than against the implementation, so a test failing means the parser disagrees
with the spec — not that the test drifted.

## One defect fixed here

**Duplicate markers in one token cancelled silently.** `{8}{#0.35}1` emitted
`AbsoluteLength` then `ResolutionChange`, and `chart_playback.rs:62-65` has
`ResolutionChange` clear the absolute length — so the `0.35s` was discarded
whichever order the author wrote them in. Same class of problem for
`(120)(240)1` and `{8}{16}1`.

Now an error, and marker emission order is fixed at BPM → length → note.
Recorded as
[ADR-0012](../../../adr/0012-one-marker-of-each-kind-per-simai-token.md) with the
two rejected alternatives. `parse_chart` gained its first reachable `Err` path;
its only caller (`systems/mod.rs:80`) already logged and skipped on `Err`.
`BIRTH.txt` has zero violations across 872 tokens, so nothing real regressed.

While there: the comment claiming the `{#S}`-before-`{N}` check order was
load-bearing is wrong — `RES_REGEX` needs a digit directly after `{` and cannot
match `{#0.35}` at all. Comment corrected, disjointness asserted directly.

## Defects found

Five, all documented by a test rather than fixed here — a bug found and fixed in
one commit leaves no evidence the test would have caught it. They are now
[`parser-defects`](../../parser-defects/spec.md), where four of the five have
shipped:

| Defect | Ticket | State |
|---|---|---|
| Unparseable token dropped, shifting the chart | [05](../../parser-defects/issues/05-parser-error-type.md) | fixed |
| Backtick pseudo-EACH unsupported | [01](../../parser-defects/issues/01-backtick-pseudo-each.md) | fixed |
| HOLD accepted only `[N:M]` | [02](../../parser-defects/issues/02-hold-duration-forms.md) | fixed |
| Partially bracketed slide chain silently inherited | [04](../../parser-defects/issues/04-partial-slide-brackets.md) | fixed |
| UTAGE `$` / `@` tap modifiers unsupported | [03](../../parser-defects/issues/03-utage-tap-modifiers.md) | todo |

Each fix shows up as the pinning test flipping from recording the bug to
asserting the notation — which is the whole reason they were written that way.

Smaller divergences pinned without a BUG label, since leniency is defensible:

- `C2` keeps `value: 2` where the doc says it behaves identically to `C`
- `12b` is accepted though the doc allows the slash-free EACH only for
  non-BREAK taps
- `v` and `V` share one arm, so `1v35` is a grand V rather than an error
- `is_ex`/`is_firework` on a slide bind only *before* the pattern, while
  `is_break` is scraped from the whole string — `1-4[8:1]x` parses and drops the
  flag. The doc gives no EX-slide example, so the reading is a judgement call
  (`slide::tests::ex_and_firework_bind_only_before_the_pattern`)
- `?`, `!` and `@` parse on a slide and are then ignored, while `$`/`@` on a tap
  fail outright — inconsistent, but both are UTAGE-only
