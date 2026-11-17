# ADR-0021 — The Catalog ships as a snapshot plus standalone reference tables; v2 has no server-side sheet search

**Status:** accepted
**Date:** 2026-11-17

## Context

[ADR-0007](0007-client-side-filtering.md) chose client-side filtering over the
whole Catalog and kept `GET /sheets/search` "retained server-side but not the
browse path". Two facts at v2 planning time:

- The search endpoint has no caller. `composables/useSheetSearch.ts` is
  imported by nothing; the browse page filters the in-memory snapshot. The
  endpoint is a module of query building with its own tests, and the CORS
  layer exposes a pagination header for it that no client reads.
- The reference tables (categories, versions, types, difficulties, regions)
  reach the client only inside the 3MB Catalog snapshot. The visualizer
  fetches its own sheet but resolves type and difficulty names through the
  snapshot's tables, and falls back to upper-cased codes when it has not
  loaded. Every community screen will have the same need, and a community
  page is a plausible first page for a visitor.

The v2 API is written from scratch and replaces `/api/v1` wholesale, so
whatever is retained is retained by choice, not by inertia.

## Decision

The Catalog is delivered as one snapshot, and the five reference tables are
also delivered on their own as a small boot-time fetch. Browse and the
upload's official-song chooser load the snapshot; every other screen fetches
one song, one sheet, one community song, or one page. There is no server-side
sheet search in v2; the browse page filters the snapshot in memory.

## Rejected alternatives

- **Keep a search endpoint beside the snapshot.** Two filter engines, one
  unused, carried into a rewrite whose point is to drop what was inherited.
  Rejected for the same reason ADR-0007 reverted it: the Catalog is small
  enough.
- **Page everything and drop the snapshot.** Removes IndexedDB, the manifest
  probe and the client-side data pass together, but needs an aggregates
  endpoint for the filter panel (designer counts, distinct levels) and
  server-side filtering, which was built once and reverted. The bigger change
  for the smaller gain, and it reopens ADR-0007 without new evidence.
- **Load the snapshot before any screen.** No new endpoint, but a community
  reader pays 3MB to see a page that needs five small tables.
- **Carry names on the wire.** Every sheet and chart payload renders its own
  `difficultyName` and `typeName`. Two sources of truth for every label.
- **A title-only search endpoint for the upload chooser.** A second filter
  path returns for one picker used by logged-in uploaders, who are few and
  whose snapshot is cached after first use.

## Consequences

The search module and its tests go with `/api/v1`; the API contract
drops them. ADR-0007's decision stands; only its "retained server-side" clause
is overtaken.

Reference tables are one more request at boot and one more thing to cache.
They are a few kilobytes and change with the Catalog, so they ride the same
revision.

The snapshot stays the only path for browse. If the Catalog ever grows past
what a phone holds comfortably, this reopens, and the first move is an
aggregates endpoint for the filter panel, not server search.

The upload chooser loads the snapshot. An uploader on a fresh device pays the
3MB before linking a chart; accepted.

Screen-level field requirements that this decision serves are in
[`docs/reference/screen-data-requirements.md`](../reference/screen-data-requirements.md).
