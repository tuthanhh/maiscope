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

**Status:** todo

Lowest value of the five. UTAGE charts are not a v1 target, and nothing in
`systems/visual/` distinguishes a star-shaped tap yet, so this makes the
notation *parse* without changing what is drawn.

- [ ] `$` and `@` accepted in `TAP_TOUCH_RE`'s modifier class
- [ ] `1$,` produces `NoteKind::SlideStar`, dropping its `#[allow(dead_code)]`
- [ ] `$$` distinguished from `$` — needs a rotation flag, or a decision to
      ignore rotation for now and say so in the ticket
- [ ] `1@-5[2:1],` keeps parsing, and the `@` stops being silently discarded
- [ ] `note::tests::utage_star_tap_modifiers_are_unsupported` inverts
