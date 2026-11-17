# 08 — Base components

**What to build:** the component inventory of *Components*:
Button (four kinds), Chip (filter and difficulty forms), Field with text
input, select and textarea, Card with jacket, level pill and bookmark mark,
Pager, Empty state, Error state (short form). Each matches its state table.

**Blocked by:** 07.

**Status:** todo

- [ ] Each component in `components/ui/`, reading tokens only
- [ ] The level pill takes `--diff` from `difficulties[].color` and falls back to `--fg`; difficulty colour never backs text
- [ ] Icon buttons always carry an accessible name
- [ ] Existing `Mv*` primitives replaced or restyled, not duplicated
