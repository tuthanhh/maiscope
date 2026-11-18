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

**2026-11-18 — review.** Applied: `aria-expanded` follows whether a list
is shown, not only whether it was asked for; `aria-selected` marks the
highlighted row, as ARIA 1.2's list autocomplete has it, and the current
value is a class; the whole list (padding, scrollbar, the "keep typing"
line) keeps the input focused, and a blur into the list is ignored for
touch browsers that skip `mousedown.prevent`; an arrow key on a closed
list opens it on the first or last row; a closed list forgets its
highlight; an opened one scrolls into the dialog's view. The scrim closes
a dialog only when the press began on it — a selection dragged out of the
artist field no longer closes the filters (the song modal shares the fix
through `useDialogFocus`). The clear button is a 28px target; the
placeholder reads "Search", not "ALL". Unverified here: iOS Safari's
blur on tap — the `relatedTarget` guard is the fallback if it misbehaves.
The choices are in design-direction's decisions table rather than an ADR:
they are UI decisions of the kind that table already holds.
