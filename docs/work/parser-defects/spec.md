# Spec — Parser defects

**Status:** active
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Fix the five simai-parser defects that
[`test-foundation` 01](../test-foundation/issues/01-engine-parser-tests.md)
found and documented. Each already has a test pinning current behaviour, so each
fix is visible as that test flipping from "records a bug" to "asserts the spec".

## Problem

Writing the parser's first tests surfaced five places where the parser and
[`docs/reference/simai-notation.md`](../../reference/simai-notation.md) disagree.
Four are unsupported notation; one is the error-handling design.

They share a failure mode: **none of them is visible at runtime.** A chart with
unsupported syntax does not fail to load — it loads with notes missing and every
later note shifted a beat early, because an unparseable token is logged to
stderr and dropped. Nothing reaches the user, and nothing reaches the logs
anyone reads.

That makes them worth fixing together and in a deliberate order.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Backtick pseudo-EACH](issues/01-backtick-pseudo-each.md) | Real charts use it |
| 02 | [HOLD accepts every duration form](issues/02-hold-duration-forms.md) | `parse_duration_bracket` already handles them |
| 03 | [UTAGE tap modifiers `$` and `@`](issues/03-utage-tap-modifiers.md) | Lowest value; out of scope for v1 rendering |
| 04 | [Reject partially bracketed slide chains](issues/04-partial-slide-brackets.md) | Blocked by nothing, but adds an error path |
| 05 | [Parser error type; refuse unparseable charts](issues/05-parser-error-type.md) | **Last.** Blocked by 01–04 |

## Decisions that constrain this work

- **Ticket 05 lands last, and the order is the point.** It turns a dropped token
  into a refused chart. Landing it before 01–04 would make every chart using
  valid-but-unsupported notation stop loading entirely — trading a silent wrong
  render for a loud total failure, on syntax the parser *should* accept. Support
  the notation first, then tighten.
- **Reject contradictions, do not guess.** Established by
  [ADR-0012](../../adr/0012-one-marker-of-each-kind-per-simai-token.md) for
  duplicate markers; ticket 04 applies the same rule to a slide chain that
  brackets some segments but not all.
- **`io::Error` goes.** Nothing in the parser touches IO. Ticket 05 replaces it
  with a parser-specific error carrying the token index, which is what makes a
  failure diagnosable from a log line.
- **Tests come from the notation doc, not the implementation.** Inherited from
  `test-foundation`; each ticket here edits an existing test rather than adding
  a parallel one, so the diff shows the behaviour change directly.

## Out of scope

Rendering the newly parsed notes. Ticket 03 in particular makes `1$,` *parse*;
whether `systems/visual/` draws a star-shaped tap differently is a separate
question, and UTAGE charts are not a v1 target.

## Done when

All five tests currently carrying a `// BUG:` comment assert the notation doc
instead, `parse_chart` returns a typed error naming the offending token, and the
`BIRTH` chart in `engine/tests/fixtures/local/` parses with zero warnings.
