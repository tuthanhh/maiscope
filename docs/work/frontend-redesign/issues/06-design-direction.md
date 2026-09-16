# 06 — Design direction

**What to build:** a document, not code. The visual system tickets 04 and 05
implement against.

This is the counterpart to [ticket 01](01-data-requirements.md): 01 says what
each screen *shows*, 06 says what it *looks like*. Both are written before any
v2 frontend code, and both land in `docs/reference/` because they outlive this
feature.

**Why it exists.** The redesign was scoped to include visual work, not just
structural (see the spec). Visual work has no natural completion test — it ends
when it looks right, which is not a date — and this feature bounds the length of
the branch that the whole v2 cutover rides on. This document is the bound. Two
alternatives were rejected: a time-box, which lands mid-screen and ships a
half-restyled app; and per-screen acceptance criteria with no shared system,
which is how screens drift apart when there is no UI framework holding them
together.

**Method.** Produced with a design skill rather than assembled by hand. The
skill's output is a *direction*; this ticket is not done until that direction is
a committed file.

**Blocked by:** None. Start alongside ticket 01.

**Status:** todo

- [ ] Type scale — families, sizes, weights, line heights, and which is used
      where
- [ ] Colour tokens, including dark mode if it is in scope, stated as tokens and
      not as hex values scattered through components
- [ ] Spacing scale, and the radius/border/elevation conventions that go with it
- [ ] Component inventory with a visual spec for each: button, card, table row,
      dialog, form field, pagination control, empty state, error state. These
      are the pieces `components/ui` already hand-rolls — there is no Vuetify,
      no Tailwind, so every one of them is ours to specify
- [ ] The community surfaces get first-class treatment, not an afterthought:
      upload form, upload *rejection* (which must render a parser error with a
      token in it), community browse, reports queue. They are new, so they have
      no existing look to inherit, and they are why the visual work is on this
      branch at all
- [ ] A worked example — one existing screen described in the new system — so
      the document is testable rather than aspirational
- [ ] Notes what is explicitly **not** changing: the visualizer's playback
      surface (spec: ticket 03 is a split, not a rework)
- [ ] Lands in `docs/reference/`, committed, before ticket 04 starts

> Copy is multiplied by `vue-i18n` and `src/locales/`. A direction that invents
> new labels invents them per language — worth knowing while writing, not after.
