# 08 — Base components

**What to build:** the component inventory of *Components*:
Button (four kinds), Chip (filter and difficulty forms), Field with text
input, select and textarea, Card with jacket, level pill and bookmark mark,
Pager, Empty state, Error state (short form). Each matches its state table.

**Blocked by:** 07.

**Status:** done

- [x] Each component in `components/ui/`, reading tokens only
- [x] The level pill takes `--diff` from `difficulties[].color` and falls back to `--fg`; difficulty colour never backs text
- [x] Icon buttons always carry an accessible name
- [x] Existing `Mv*` primitives replaced or restyled, not duplicated

## Comments

**2026-11-17 — built.** `MvButton` (four kinds, optional
`to`), `MvChip` (restyled; `diff` switches to the underline form), `MvField`
(restyled in place, same `label` API, plus help and error), `MvTextInput` and
`MvSelect` (restyled; shared `_control.scss` state table), `MvTextarea`,
`MvLevelPill`, `MvCard`, `MvPager`, `MvEmpty`, `MvError`; new icons for the
pager, designer line, share, shuffle and navigation. Checked in light and
dark through a throwaway harness route that was not committed. Tickets
09–12 adopted them, and added `MvDrawer`, `MvModal` and `MvLevelLine`.

**2026-11-17 — review** (applied in `84f751b`). `MvField` takes its id from
`useId` and provides it to the control inside; a linked `MvButton` forwards
its click; an icon button without a `label` warns in development; the pager
names itself and keeps its range on one line; a control's focus outline
shows on `:focus-visible` only; the card's designer name ellipsises.
