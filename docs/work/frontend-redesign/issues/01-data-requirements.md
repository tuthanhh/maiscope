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

**Status:** todo

- [ ] Per screen: the fields displayed, and which are derived today
      (`songNo`, `imageUrl`, `sheetExpr`, `notePercents`, `$canonicalSheet`)
- [ ] For each derived field, a recommendation: server-side, client-side, or
      gone — with the reason. `imageUrl` needs `dataSourceUrl`, which is client
      config; `notePercents` needs only `noteCounts`, which the server has
- [ ] Which screens need the full catalog and which need a page of results —
      the question [ADR-0007](../../../adr/0007-client-side-filtering.md) answered
      for v1, worth re-asking now that a second library exists
- [ ] What the community screens need, so `community-charts` payloads are
      designed against the same list
- [ ] Lands in `docs/reference/` — it outlives this feature and
      `api-rewrite` reads it

> Resist designing the API while writing this. The output is *what the screen
> shows*, not what an endpoint returns. Mixing them produces a list that
> justifies the API you already had in mind.
