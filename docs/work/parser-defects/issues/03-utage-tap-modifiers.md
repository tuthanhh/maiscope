# 03 — UTAGE tap modifiers `$` and `@`

**What to build:** accept the star-shape modifiers on a TAP.

Per [Other Notations](../../../reference/simai-notation.md#other-notations):

- `1$,` — force the star-shaped TAP that normally only appears with a SLIDE
- `1$$,` — same, but the star rotates
- `1@,` — revert a SLIDE's automatic star back to a normal TAP

None of `$` or `@` is in `TAP_TOUCH_RE`'s modifier class, so all three fail.

The gap is inconsistent rather than uniform: `SLIDE_RE` *does* admit
`[xfb@?!$]*` before the pattern and then ignores what it finds, so `1@-5[2:1],`
parses fine while `1@,` does not.

`NoteKind::SlideStar` already exists and is `#[allow(dead_code)]` — this is what
would use it.

**Blocked by:** None.

**Status:** done — 2026-11-16

Lowest value of the five. UTAGE charts are not a v1 target, so this makes the
notation *parse*; what is drawn changes only where the renderer already had the
sprite to hand (see Comments).

- [x] `$` and `@` accepted in `TAP_TOUCH_RE`'s modifier class
- [x] `1$,` produces `NoteKind::SlideStar`, dropping its `#[allow(dead_code)]`
- [x] `$$` distinguished from `$` — `SlideStar { button, rotates }`; the flag
      is recorded and not drawn. [ADR-0017](../../../adr/0017-utage-star-modifiers-as-flags.md)
- [x] `1@-5[2:1],` keeps parsing, and the `@` stops being silently discarded —
      `NoteKind::Slide` gained `plain_head: bool`
- [x] `note::tests::utage_star_tap_modifiers_are_unsupported` inverts — split
      into four tests: the two star forms, their combination with `b`/`x`, `@`
      as a no-op on a bare TAP, and the rejected combinations

## Comments

**What the renderer does with it.** `NoteKind::SlideStar` already had a spawn
arm (`spawning.rs`), so `1$,` draws as a star today with no renderer change;
`$$` draws identically because nothing spins yet. `@` needed one change: the
slide head sprite is now chosen by `plain_head` — tap sprite under `@`, star
otherwise — through an `Option<Sprite>` that replaced the `has_head: bool`
parameter on `spawn_slide_children` and `spawn_fan_children`. That is the whole
of the visual delta.

**Undefined combinations are rejected, not guessed at**, following
[ADR-0012](../../../adr/0012-one-marker-of-each-kind-per-simai-token.md): `$$$`,
`$` together with `@`, `$` on a HOLD, `$` on a TOUCH, and `$` on the two-digit
EACH shorthand (`12$` — write `1$/2$`). Each error names the rule it broke.

**`@` on a note that was never a star is a no-op**, and accepted as one. The
notation only defines it on a slide head, but the ticket's own acceptance
criterion (`1@,` parses) rules out rejecting it, and "revert a star that is not
there" has an obvious meaning.

**`$` on a slide head** (`1$-5[8:1],`) is admitted by `SLIDE_RE` and ignored,
since the head is already a star. `?` and `!` are unchanged: admitted, ignored,
pinned by `slide::tests::star_suppression_modifiers_parse_but_are_ignored`.

**Fixture.** `modifiers.txt` gained the four spellings; the snapshot moved from
12 to 16 notes, exactly the four added. `E` has to stay the last line — the
parser folds everything after it into one token, which is how the first attempt
at this produced a `E1$` touch and a failing corpus.
