# 07 — Tokens and type

**What to build:** the token layers of
[`design-direction.md`](../../../reference/design-direction.md) *Tokens* —
primitives, semantic (light and dark), component tokens — as CSS custom
properties on `:root`, and the two type families with the seven roles of the
type scale as utility classes.

**Blocked by:** 04.

**Status:** todo

- [ ] Primitive, semantic and component tokens exactly as the tables list them; components read semantic or component tokens only
- [ ] Dark mode swaps the semantic layer and nothing else (the existing `.dark` class on `<html>`)
- [ ] `--f-sans` (Inter, Latin subset, plus the system CJK stack) and `--f-mono` (JetBrains Mono)
- [ ] The seven type roles as classes (`t-label`, `t-meta`, `t-body`, `t-strong`, `t-heading`, `t-title`, `t-display`), phone sizes for `title` and `display`, `palt` for Japanese in the sans roles
- [ ] `<html lang>` follows the active locale
- [ ] The old `--mv-*` tokens map onto the new semantic ones for the screens not yet restyled, and are deleted by the last screen ticket
- [ ] Focus is `1px solid var(--focus)` offset 2px on `:focus-visible`; `prefers-reduced-motion` honoured
