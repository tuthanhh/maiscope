# 16 — Filters dialog and artist picker

**What to build:** the maintainer's asks of 2026-11-18 about the filters.
On desktop the filters open as a dialog floating in the middle of the
screen instead of a drawer against the edge; on phones they stay a sheet
from the bottom. Artist is chosen from the catalog's artists: a list that
drops open on focus like a dropdown, narrowed as you type, with free text
still allowed.

**Blocked by:** 10.

**Status:** done

- [x] Desktop: centred, `--modal-w`-wide dialog, at most 80vh, scrolling
      inside; same heading, Reset and close; phones unchanged
- [x] Artist: a combobox (ARIA 1.2 pattern) over every artist in the
      Catalog, sorted; opens on focus or click with the full list, filters as
      you type, arrows and Enter choose, Escape closes the list (not the
      dialog); a typed value that matches no artist still filters by substring
- [x] Exact match keeps working with a chosen artist
- [x] design-direction's "Drawer" section updated

## Comments

**2026-11-18 — built.** `MvDrawer` keeps its name and API; on desktop the
scrim centres it as a dialog, on phones it is the same bottom sheet.
`MvCombobox` is new in `components/ui/`: ARIA 1.2 combobox with list
autocomplete, `aria-activedescendant` for the highlighted row, options
chosen on `mousedown` so the input never blurs first. Artist options are
each Catalog artist once, sorted with `localeCompare`; prefix matches rank
first. A string "Keep typing to narrow the list" joins the nine locales.
