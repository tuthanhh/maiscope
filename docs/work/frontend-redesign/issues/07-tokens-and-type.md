# 07 — Tokens and type

**What to build:** the token layers of
[`design-direction.md`](../../../reference/design-direction.md) *Tokens* —
primitives, semantic (light and dark), component tokens — as CSS custom
properties on `:root`, and the two type families with the seven roles of the
type scale as utility classes.

**Blocked by:** 04.

**Status:** done

- [x] Primitive, semantic and component tokens exactly as the tables list them; components read semantic or component tokens only
- [x] Dark mode swaps the semantic layer and nothing else (the existing `.dark` class on `<html>`)
- [x] `--f-sans` (Inter, Latin subset, plus the system CJK stack) and `--f-mono` (JetBrains Mono)
- [x] The seven type roles as classes (`t-label`, `t-meta`, `t-body`, `t-strong`, `t-heading`, `t-title`, `t-display`), phone sizes for `title` and `display`, `palt` for Japanese in the sans roles
- [x] `<html lang>` follows the active locale
- [x] The old `--mv-*` tokens map onto the new semantic ones for the screens not yet restyled
- [ ] …and are deleted by the last screen ticket (13)
- [x] Focus is `1px solid var(--focus)` offset 2px on `:focus-visible`; `prefers-reduced-motion` honoured

## Comments

**2026-11-17 — built.** `assets/styles/tokens.scss` holds the three layers;
`_breakpoints.scss` has the SCSS twins of `--bp-md`/`--bp-lg`, since media
queries cannot read custom properties. One semantic token was added beyond
the table: `--inverse-hover` (`--n-700` light, `--n-300` dark), the primary
button's hover background the Button table specifies per mode. Inter and
JetBrains Mono load from Google Fonts, as JetBrains Mono already did; the CJK
names stay system fonts. `<html lang>` follows the locale's ISO code.

The v1 `--mv-*` names are re-pointed at the semantic layer, so every
unrestyled screen already takes the new palette — including the muted grey
raised to pass AA (`#999` → `#767676`). Checked in headless Chromium in both
modes; the dark class swaps only the semantic layer.
