# ADR-0017 — UTAGE star modifiers are flags on the existing note kinds

**Status:** accepted
**Date:** 2026-11-16

## Context

simai's UTAGE notations ([`simai-notation.md` — Other Notations](../reference/simai-notation.md#other-notations))
let a chart override the star shape a TAP would otherwise get:

- `1$,` forces the star-shaped TAP that normally only appears as a slide head
- `1$$,` is the same star, rotating at a fixed speed
- `1@-5[8:1],` reverts a slide head from the automatic star to a normal TAP

None of them parsed on a TAP, and `@` on a slide parsed and was discarded
([`parser-defects` 03](../work/parser-defects/issues/03-utage-tap-modifiers.md)).
The parser's data model already had `NoteKind::SlideStar(ButtonId)` — "a star
head with no path" — marked `dead_code` because nothing produced it, and
`NoteKind::Slide` had no way to say what its head looks like.

The question was how to represent the three forms without disturbing the
renderer, which draws slide heads as stars in two places and knows nothing about
UTAGE.

## Decision

**Represent each modifier as a field on the note kind it modifies, carried
whether or not the renderer draws it yet.**

- `SlideStar(ButtonId)` becomes `SlideStar { button, rotates }`. `$` sets
  `rotates: false`, `$$` sets it `true`. The renderer draws both as the same
  star; the flag is recorded so that adding the spin is a renderer change, not
  a parser one.
- `Slide` gains `plain_head: bool`, set by `@`. The renderer picks the head
  sprite from it — tap under `@`, star otherwise — which is the entire visual
  consequence of this ADR.
- `@` on a note that was never a star (`1@,`) is a no-op and parses as the
  bare note.

Combinations the notation does not define are errors, not guesses, as
[ADR-0012](0012-one-marker-of-each-kind-per-simai-token.md) established for
duplicate markers: `$$$`, `$` with `@`, `$` on a HOLD, `$` on a TOUCH, and `$`
on the two-digit EACH shorthand.

## Rejected alternatives

**Parse `1@-5[8:1]` as two notes, `Tap(1)` plus a `HeadlessSlide`.** Zero
changes to the data model and the renderer would draw it correctly for free: a
tap sprite approaching button 1, and a headless track traced from it. Rejected
because it is not what the notation says. `@` changes how one note *looks*;
"the SLIDE itself still plays out normally". Splitting it makes the slide's
head a separate note with its own judgement, its own EACH pairing and its own
place in the note count, and every later consumer — the v2 community-chart
tooling in particular — would have to know that two notes are sometimes one.
A flag says what happened; two notes say something else happened.

**Collapse `$$` into `$` and drop rotation.** The ticket offered this as an
option provided it was written down. Rejected because the cost of keeping the
distinction is one `bool`, while dropping it means the parser silently reads
two spellings as one — the exact failure mode `test-foundation` 01 was written
to stop. A `contains('$')` check would have done it by accident; the count is
asserted in `note::tests::dollar_makes_a_star_tap_and_two_make_it_rotate`.

**Accept every combination and let the renderer sort it out.** `1$$$` as a
rotating star, `1$@` as whichever wins. Rejected on ADR-0012's grounds: a token
that says two contradictory things has a mistake in it, and the parser should
say so rather than pick.

## Consequences

`NoteKind::SlideStar` and `NoteKind::Slide` changed shape, so every
constructor and pattern in `systems/visual/` and the parser tests was touched.
Both are `pub` through the `engine::chart` façade; `tests/corpus.rs` only counts
events and did not care.

`$$` is now a promise the renderer does not keep. Anyone adding the spin finds
the flag waiting; anyone reading a `$$` chart in the visualizer today sees a
still star and should expect to.

`?` and `!` (star suppression on a slide) are still admitted and discarded.
This ADR is the pattern for representing them when they are wanted: a field on
`Slide`, not a new note kind. They were left out because the ticket did not
cover them and no fixture uses them.

`plain_head` reaches the renderer through an `Option<Sprite>` where a
`has_head: bool` used to be, so a headless slide and a plain-headed slide are
now distinguishable at the spawn site — which is also where the next head
variant would plug in.
