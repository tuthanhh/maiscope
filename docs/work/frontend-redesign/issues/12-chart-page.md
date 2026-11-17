# 12 — Chart page

**What to build:** the *Chart page*: the playback surface
unchanged, the title block under it, the chart text as a side column from
`--bp-lg` and stacked below it, other difficulties, bookmark and share as text
buttons; and the boot fetch of `/api/reference`
([ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md))
so names resolve without the Catalog.

**Blocked by:** 08, 11.

**Status:** todo

- [ ] Playback surface (canvas, transport, sliders, `?debug`) untouched
- [ ] Difficulty colour from the reference table, not the hard-coded map
- [ ] Names resolve from `/api/reference` fetched at boot, before and without the Catalog
- [ ] Publish appears only with `community-charts` (frontend-redesign 05)
