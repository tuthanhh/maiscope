# 12 — Chart page

**What to build:** the *Chart page*: the playback surface
unchanged, the title block under it, the chart text as a side column from
`--bp-lg` and stacked below it, other difficulties, bookmark and share as text
buttons; and the boot fetch of `/api/reference`
([ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md))
so names resolve without the Catalog.

**Blocked by:** 08, 11.

**Status:** done

- [x] Playback surface (canvas, transport, sliders, `?debug`) untouched
- [x] Difficulty colour from the reference table, not the hard-coded map
- [x] Names resolve from `/api/reference` fetched at boot, before and without the Catalog
- [ ] Publish appears only with `community-charts` (frontend-redesign 05)

## Comments

**2026-11-18 — built.** The page is a grid: stage, title block, chart text;
from `--bp-lg` the chart text is a `--drawer-w` side column beside the
stage and the height of it. `VizTitleBlock` replaces `VizSheetCard`: title
and level (in `--diff` from the reference table — the hard-coded colour map
is gone), the meta line, Bookmark and Share as text buttons, the jacket with
designer, note counts and type · version · date, then the other
difficulties as links. Share uses the system share sheet where there is
one and the clipboard otherwise. A link to another difficulty is the same
route, so the page now reloads on a changed deep link. `VizPastePanel` is
an `MvField` with a mono `MvTextarea` and a primary Load. The store fetches
`/api/reference` at boot; its lookup maps read those tables until the
Catalog installs. The YouTube search is gone, with `getSheetSearchLink`.

Departures:

- **No app-wide loading overlay.** It covered the chart page while the
  Catalog loaded, hiding the names this ticket makes available without it.
  Browse shows "Loading data…" instead; `design-direction.md` and
  `screen-data-requirements.md` are updated. `MvLoadingOverlay` is deleted.
- **Other difficulties need the Catalog**: the single-sheet fetch carries
  no siblings, so the section is absent until the Catalog loads.
- **One sizing change to the playback surface**: the slider track's minimum
  width drops from 150px to 96px, so the row fits a 390px phone. It
  overflowed before this ticket too.
- Publish waits for 05, as the checklist says.
