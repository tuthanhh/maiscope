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

## Scaffold notes

Tests are scaffolded and the harness verified green; the per-case assertions are
still to write. 5 exemplar tests pass, 35 stubs are `#[ignore = "TODO"]`, each
carrying a doc comment naming exactly what to assert.

Decisions taken while scaffolding:

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
[`docs/reference/simai-notation.md`](../../reference/simai-notation.md) rather
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
[ADR-0012](../../adr/0012-one-marker-of-each-kind-per-simai-token.md) with the
two rejected alternatives. `parse_chart` gained its first reachable `Err` path;
its only caller (`systems/mod.rs:80`) already logged and skipped on `Err`.
`BIRTH.txt` has zero violations across 872 tokens, so nothing real regressed.

While there: the comment claiming the `{#S}`-before-`{N}` check order was
load-bearing is wrong — `RES_REGEX` needs a digit directly after `{` and cannot
match `{#0.35}` at all. Comment corrected, disjointness asserted directly.

## Defects found and left unfixed

Documented by a test rather than fixed — a bug found and fixed in one commit
leaves no evidence the test would have caught it.

1. **A failed token drops its event and shifts the chart** (`chart.rs:46-52`).
   An unparseable token is logged to stderr and *no* event is pushed, so every
   note after it lands one beat early. Worse than first recorded: `parse_chart`
   returns `Ok` with an empty vec for a chart that parsed to nothing, so a caller
   cannot distinguish that from an empty chart. Pinned by
   `unparseable_token_drops_the_event_and_shifts_the_chart`. The fix is a design
   choice — skip, emit `Rest`, or propagate `Err` — and should land together
   with a parser-specific error type replacing `io::Error`, per ADR-0012's
   consequences.
2. **Backtick pseudo-each is unsupported.** The `BIRTH` fixture fails three
   tokens: `` Invalid note syntax: 'E6`B5' ``. The `` ` `` separator appears in
   neither `TAP_TOUCH_RE` nor the slide tokenizer. Because of defect 1 this is
   silent — three note groups vanish from a real chart. Pinned by
   `note::tests::pseudo_each_backtick_is_unsupported`.
3. **HOLD accepts only `[N:M]`.** `TAP_TOUCH_RE` hard-codes
   `\[(\d+):(\d+)\]`, so the notation doc's `4h[#5.678],` and `4h[150#2:1],`
   are both rejected. `parse_duration_bracket` already handles the second; the
   regex never gives it the chance. Pinned by
   `note::tests::hold_rejects_non_simple_durations`.
4. **UTAGE `$` / `@` tap modifiers are unsupported.** `1$,` (force star-shaped
   TAP) and `1@,` fail, while `@`, `?` and `!` *do* parse on a slide and are
   then ignored — so the gap is inconsistent rather than uniform. Lowest
   priority: UTAGE is out of scope for v1. Pinned by
   `note::tests::utage_star_tap_modifiers_are_unsupported`.

5. **A partially bracketed slide chain silently inherits.** The doc is explicit
   that when per-segment lengths are given, "every sub-track needs its own
   length — omitting one causes an error". `1-4[2:1]q7-2[1:1]` instead fills the
   bracket-less segment from the last one and marks the note
   `shared_duration: true`, so it traces at a speed the author never wrote.
   Pinned by `slide::tests::partially_bracketed_chain_silently_inherits`.
   Rejecting it means distinguishing "only the last is bracketed" from "some are
   bracketed" in `parse_chained_slide_segments` — a real change, and no fixture
   exercises it yet.

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
