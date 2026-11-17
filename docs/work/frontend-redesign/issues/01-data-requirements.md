# 01 — Data requirements per screen

**What to build:** a document, not code. For each screen, the fields it renders.

This is the long pole of the whole v2 effort.
[`api-rewrite`](../../api-rewrite/spec.md) cannot decide its payload without it:
whether the server sends raw fields or rendered ones depends entirely on what
the client renders, and the current answer — raw only, with `preprocessData`
deriving the rest — was inherited from arcade-songs rather than chosen.

Screens today: `home`, `songs` (browse), `song` (detail), `visualizer`, `about`,
plus `MvSheetDialog`. v2 adds community browse, upload, and a reports queue.

**Blocked by:** None. Start here.

**Status:** done (2026-11-17) — [`docs/reference/screen-data-requirements.md`](../../../reference/screen-data-requirements.md)

- [x] Per screen: the fields displayed, and which are derived today
      (`songNo`, `imageUrl`, `sheetExpr`, `notePercents`, `$canonicalSheet`)
- [x] For each derived field, a recommendation: server-side, client-side, or
      gone — with the reason. `imageUrl` needs `dataSourceUrl`, which is client
      config; `notePercents` needs only `noteCounts`, which the server has
- [x] Which screens need the full catalog and which need a page of results —
      the question [ADR-0007](../../../adr/0007-client-side-filtering.md) answered
      for v1, worth re-asking now that a second library exists
- [x] What the community screens need, so `community-charts` payloads are
      designed against the same list
- [x] Lands in `docs/reference/` — it outlives this feature and
      `api-rewrite` reads it

> Resist designing the API while writing this. The output is *what the screen
> shows*, not what an endpoint returns. Mixing them produces a list that
> justifies the API you already had in mind.

## Comments

**2026-11-17 — grilled after the first draft; eleven decisions taken.**

Snapshot plus standalone reference tables, no server search in v2
([ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md)).
The rest, reversible and recorded in the reference document: Catalog nests on
the wire with an explicit `song` reference replacing the prototype chain;
cover URL client-side in one helper; bookmarks persist per browser by minted
sheet id, per-account later; `difficulties[].color` is the one colour source
and no other unrendered field is required; uploader login shown on community
browse and detail; a linked fan chart carries and borrows its official song's
title and cover; the visualizer takes `?sheet=` or `?communityChart=`; song
detail is a per-song fetch; the upload chooser searches the snapshot.
