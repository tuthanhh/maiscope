# 16 — Filters dialog and artist picker

**What to build:** the maintainer's asks of 2026-11-18 about the filters.
On desktop the filters open as a dialog floating in the middle of the
screen instead of a drawer against the edge; on phones they stay a sheet
from the bottom. Artist is chosen from the catalog's artists: a list that
drops open on focus like a dropdown, narrowed as you type, with free text
still allowed.

**Blocked by:** 10.

**Status:** todo

- [ ] Desktop: centred, `--modal-w`-wide dialog, at most 80vh, scrolling
      inside; same heading, Reset and close; phones unchanged
- [ ] Artist: a combobox (ARIA 1.2 pattern) over every artist in the
      Catalog, sorted; opens on focus or click with the full list, filters as
      you type, arrows and Enter choose, Escape closes the list (not the
      dialog); a typed value that matches no artist still filters by substring
- [ ] Exact match keeps working with a chosen artist
- [ ] design-direction's "Drawer" section updated
