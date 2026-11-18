# 12 — Chart page

**What to build:** the *Chart page*: the playback surface
unchanged, the title block under it, the chart text as a side column from
`--bp-lg` and stacked below it, other difficulties, bookmark and share as text
buttons; and the boot fetch of `/api/reference`
([ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md))
so names resolve without the Catalog.

**Blocked by:** 08, 11.

**Status:** done

- [x] Playback surface (canvas, transport, sliders, `?debug`) untouched — the controls are; their box moved (see departures)
- [x] Difficulty colour from the reference table, not the hard-coded map
- [x] Names resolve from `/api/reference` fetched at boot, before and without the Catalog
- [ ] Publish appears only with `community-charts` (frontend-redesign 05) — mirrored on 05's checklist

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
- **Two sizing changes to the playback surface**: the slider track's
  minimum width drops from 150px to 96px, so the row fits a 390px phone (it
  overflowed before this ticket too); and the stage's own 20px padding and
  right border give way to the page grid's padding and gap, so the canvas
  box sits where the grid puts it.
- Publish waits for 05, as the checklist says.

**2026-11-18 — review.** Applied: a deep-link load superseded while in
flight no longer wins — a stale chart response is dropped, and only the
current load clears `loading` or sets the error. The page reloads on a
change of the deep link itself (one watched key), only while it is the
route, so the render toggles do nothing and Back to a song-modal URL does
not load a chart on a page being torn down; a pasted chart is never
overwritten by a re-fetch of the URL's chart. The title block stays mounted
between sheets and, when the followed link vanishes, focus moves to the new
title. Share falls back from a refused system sheet to the clipboard, and
from a missing clipboard API (plain http) to `execCommand`; its note timer
is cleared and reset. A field error is `role="alert"`. The page has a
visually hidden h1 before a chart loads (`.sr-only` joins `tokens.scss`).
The reference tables are fetched again by a Catalog retry when the boot
fetch failed. Already handled by 13: the orphaned `page.viz.*` and
`searchOnYouTube` keys. Noted, not changed: `?msaa`, `?scale` and `?lead`
still apply on mount only.
