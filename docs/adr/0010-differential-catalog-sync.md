# ADR-0010 — Differential catalog sync, never deleting songs or sheets

**Status:** accepted
**Date:** 2026-09-14

## Context

`bin/ingest` refreshed the catalog with `TRUNCATE catalog_meta, categories,
versions, types, difficulties, regions, songs RESTART IDENTITY CASCADE`. The
cascade did not stop at the catalog: `sheets.song_id_fk` and `charts.sheet_id`
are both `ON DELETE CASCADE`, so **every catalog refresh destroyed all chart
text** — the one asset this project uniquely holds, and the one
[ADR-0001](0001-postgres-source-of-truth-for-charts.md) names Postgres the
source of truth for.

Two further consequences made a scheduled refresh untenable. `bin/ingest` bumped
`last_full_reload_revision` on every run, so `GET /sync/delta?since=N` answered
`409` for any earlier `N` and every client refetched the full 4.7MB daily —
exactly the cost the sync tier was built to remove. And a truncate driven by a
third party's bucket, on a cron, with nobody watching, is an outage waiting for a
bad upstream response.

The schema already anticipated the fix: `songs.revision`, `sheets.revision`,
`deleted_songs` and `deleted_sheets` existed, the last two commented "populated
once a deletion capability is added later".

## Decision

Replace truncate-and-reload with `bin/sync_catalog`: fetch upstream `data.json`,
sanity-gate it, diff it against the canonical tables, and write only what
changed. **No statement in the sync may `DELETE` from `songs`, `sheets` or
`charts`** — a row that vanishes upstream is recorded in
`deleted_songs`/`deleted_sheets` and left in place, so chart loss is impossible
by construction rather than by a rule someone has to remember. `bin/ingest` is
deleted outright.

Scope of that guarantee, precisely: the five lookup tables (`categories`,
`versions`, `types`, `difficulties`, `regions`) are still replaced wholesale,
and sheet sub-table rows are deleted scoped to an explicit dirty-sheet list.
Neither cascades into chart text.

Delivered by the shipped `catalog-sync` feature (5 tickets). The full decision
record — sanity thresholds, the `catalog_meta` row lock, set-based statement
batching, sub-table reconciliation — stays in
[`work/catalog-sync/spec.md`](../work/catalog-sync/spec.md); this ADR records
only the choice that constrains everything else.

## Rejected alternatives

- **Delete rows that have no chart attached.** Safe today, but conditional: a
  future foreign-key change breaks the guarantee silently, and nothing fails
  loudly when it does.
- **Mirror upstream exactly.** Restores the data-loss path this ADR exists to
  close.
- **Keep `bin/ingest` behind a flag.** Its only remaining use was "rebuild from
  scratch", which the sync does anyway against an empty database. Keeping it
  preserves a footgun for a case that no longer exists.
- **A write endpoint instead of a binary.** v1 has no write API
  ([ADR-0002](0002-chart-data-only-no-audio-hosting.md)); adding one for this
  would be a far larger surface than a scheduled `bin/` tool.

## Consequences

`last_full_reload_revision` is now never written. That is what makes delta sync
survive a refresh, and it is the single line most likely to be reintroduced by
accident — `routes/sync.rs:66` carries a comment saying so.

The catalog drifts from upstream over time: a song dropped upstream years ago
stays visible in `/catalog`, indistinguishable from a current one. Accepted
deliberately — a `hidden` flag filtered from `/catalog` is the better end state
but is not worth the schema and query surface until the drift is an observed
problem. Observed drift would reopen this.

A run that changes nothing bumps no revision at all, including
`catalog_meta.revision`, so the `/catalog` ETag holds and clients do not
refetch. This is load-bearing for the daily cron: a bump on a no-op run would
reintroduce the daily 4.7MB cost in a different place.
