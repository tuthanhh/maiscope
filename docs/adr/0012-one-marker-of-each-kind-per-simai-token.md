# ADR-0012 — One marker of each kind per simai token

**Status:** accepted
**Date:** 2026-09-16

## Context

A simai token — the text between two commas — may carry meta markers as well as
a note: `(B)` sets the BPM, `{N}` the length divider, `{#S}` the per-comma
length directly in seconds. `(120){8}1` is a BPM change, a divider change and a
tap, all on one comma.

Writing the parser's first tests
([`test-foundation` 01](../work/test-foundation/issues/01-engine-parser-tests.md))
turned up two questions the code answered only by accident.

**Emission order did not follow the source.** `strip_meta_tokens` ran three
independent regex passes in a loop and pushed events in *check* order, so
`(120){8}1` and `{8}(120)1` both emitted `[BpmChange, ResolutionChange]`. The
notation requires BPM before the divider — a note length cannot be computed
without a BPM — so the normalisation happened to be right, but nothing recorded
it as intentional.

**Duplicate markers silently cancelled.** `{#S}` was checked before `{N}`, so
`AbsoluteLength` was always emitted first, and
[`chart_playback.rs`](../../engine/src/systems/chart_playback.rs) has
`ResolutionChange` clear the absolute length. Both `{8}{#0.35}1` and
`{#0.35}{8}1` therefore discarded the 0.35s and fell back to the
`240 / BPM / divider` formula. An author writing either plainly meant "this
comma is 0.35 seconds long" and got something else, with nothing logged.

The same held for `(120)(240)1` and `{8}{16}1`: one marker won, and which one
was an artefact of the matching order rather than of anything the author wrote.

A comment in the parser claimed the `{#S}`-before-`{N}` check order was
load-bearing, because `{#0.35}` would otherwise be partly eaten by the
integer-only resolution pattern. It is not: `RES_REGEX` is `\{(\d+)\}` and
requires a digit directly after `{`, so it cannot match `{#0.35}` at all. The
two patterns are disjoint and the order was free all along.

## Decision

**Within one token, each marker kind may appear at most once. A duplicate is an
error.** `{N}` and `{#S}` count as the *same* kind, since both set the per-comma
length and `{#S}` explicitly replaces the divider.

**Markers emit in a fixed order regardless of how they were written:** BPM, then
the length marker, then the note.

`strip_meta_tokens` became a pure function returning
`Result<(String, Vec<ChartEvent>), String>`, and `parse_chart` propagates the
failure as `io::ErrorKind::InvalidData`. The only caller
([`systems/mod.rs`](../../engine/src/systems/mod.rs)) already logged and skipped
on `Err`, so a rejected chart degrades to "this song does not load" rather than
taking down the canvas.

The restriction is per token, not per chart: redefining BPM or the divider in a
*later* token is ordinary notation and stays legal, including switching out of
absolute-length mode with a subsequent `{N}`.

## Rejected alternatives

**Order the markers so the conflict resolves in the author's favour** — emit
`AbsoluteLength` *after* `ResolutionChange` so `{#S}` wins when both appear.
One-line change, and it makes `{8}{#0.35}1` do the more plausible thing. But it
still silently picks a winner for input that has no defined meaning, and it
picks it by a rule invisible in the chart text. A chart that says two
contradictory things about one comma is a chart with a mistake in it; the parser
should say so rather than guess well.

**Preserve source order within the token.** Most faithful to what was written:
scan left to right and emit in that order, so `{8}{#0.35}` and `{#0.35}{8}` mean
different things. Rejected because it also drops the BPM-before-divider
normalisation, which is wanted — the notation *requires* that order, so
accepting the reversed spelling and fixing it up is a kindness, not a liberty.
It is also a real rewrite of `strip_meta_tokens` rather than a guard.

**Leave it and document the quirk in a test.** Zero behaviour change, zero risk
to charts that already load. Rejected because the failure is invisible: no log,
no error, just a chart playing at the wrong speed. `BIRTH.txt` happens to have
zero violations across its 872 tokens, so nothing real is broken by rejecting
them — the cost of being strict is currently zero and rises the longer it waits.

## Consequences

`parse_chart` now has a reachable `Err` path for the first time. Its signature
already promised `Result<_, io::Error>`, so no caller changed.

`io::Error` remains a poor fit — nothing here touches IO — but replacing it with
a parser-specific error type is a wider change than this ADR covers, and would
need to account for the *other* error path in `parse_chart`, which is still
wrong: a token whose note fails to parse is logged to stderr and dropped
entirely, silently shifting every later note one beat early. That defect is
recorded in
[`test-foundation` 01](../work/test-foundation/issues/01-engine-parser-tests.md)
and pinned by
`chart::tests::unparseable_token_drops_the_event_and_shifts_the_chart`. The two
should be fixed together, behind one error type.
