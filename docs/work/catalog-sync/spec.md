# Spec — Differential catalog sync

**Status:** planned
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Replace truncate-and-reload with a scheduled differential sync: fetch upstream
`data.json`, compare it against the canonical tables, and update only what
changed. Never delete.

## Problem

`bin/ingest` runs `TRUNCATE catalog_meta, categories, versions, types,
difficulties, regions, songs RESTART IDENTITY CASCADE`. The cascade is not
confined to the catalog:

```
TRUNCATE songs ... CASCADE
  -> sheets   (song_id_fk BIGINT NOT NULL REFERENCES songs(id) ON DELETE CASCADE)
    -> charts (sheet_id   BIGINT NOT NULL REFERENCES sheets(id) ON DELETE CASCADE)
```

**Every catalog refresh destroys all chart text** — the one asset this project
uniquely holds, and the one [ADR-0001](../../adr/0001-postgres-source-of-truth-for-charts.md)
names Postgres the source of truth for. Recovery means re-running `seed_songs`
from the private data repo, so the loss is currently repairable; it stops being
repairable the moment anything enters `charts` that did not come from that repo.

Two further consequences of reloading rather than diffing:

- **Delta sync never works.** `bin/ingest` bumps `last_full_reload_revision` on
  every run, and `GET /sync/delta?since=N` answers `409` for any `N` below it. A
  daily refresh means every client refetches the full 4.7MB daily, which is the
  exact cost the sync tier was built to avoid.
- **Refresh cannot be automated safely.** A truncate driven by a third party's
  bucket, on a schedule, with no human watching, is an outage waiting for a bad
  upstream response.

The schema already anticipates the fix. `songs.revision`, `sheets.revision`,
`deleted_songs` and `deleted_sheets` exist, the last two with the comment
"populated once a deletion capability is added later". This feature is that
later.

## Measured facts

Against the live feed on 2026-09-13 (`https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json`):

| | |
|---|---|
| payload | 4.7 MB |
| songs | 1,845 |
| sheets | 7,348 |
| `songId` null or absent | 0 |
| duplicate `songId` | 0 |
| duplicate `sheet_expr` | 0 |
| `updateTime` | 2026-09-11T01:24:07Z |

Natural keys are therefore usable directly: `songs.song_id UNIQUE`,
`sheets.sheet_expr UNIQUE`.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | Extract `RawData` types out of `bin/ingest.rs` into a shared module | They are private to that binary today |
| 02 | `bin/sync_catalog`: fetch, sanity-gate, diff, upsert, log vanished rows | The core of this feature |
| 03 | Scheduled workflow — daily cron plus `workflow_dispatch` | |
| 04 | Delete `bin/ingest` | Removes the cascade path entirely |
| 05 | Tests: first sync, no-op sync, changed field, vanished row, sanity gate | `#[sqlx::test]`, no live network |

## Decisions that constrain this work

- **Never delete, structurally.** A row that vanishes upstream is recorded in
  `deleted_songs`/`deleted_sheets` and left in place. `bin/sync_catalog` contains
  no `DELETE` statement, so chart loss is impossible by construction rather than
  by a rule someone must remember. *Rejected:* deleting rows with no chart
  attached (safe, but conditional — a future FK change breaks the guarantee
  silently), and mirroring upstream exactly (restores the data-loss path).
- **Vanished rows stay visible in `/catalog`.** No `hidden` column, no query
  changes, no migration — the sync only ever inserts and updates. *Accepted cost:*
  the catalog drifts from upstream over time, with no way to distinguish a current
  song from one dropped years ago. *Rejected:* a flag filtered from `/catalog`
  except where chart text exists, which is the better end state but is not worth
  the schema and query surface until drift is an observed problem.
- **Sanity thresholds before any write.** Abort if the songs array is empty, or if
  the incoming song count is below 90% of the current `COUNT(*)` from `songs`
  (skipped when the table is empty, so a first sync is not blocked by its own
  threshold). The job exits
  non-zero and writes nothing. *Rejected:* trusting serde plus the transaction —
  which does prevent partial writes, but not garbage that happens to parse; and a
  staging-table diff, which is more machinery than a catalog changing a few rows a
  month deserves.
- **A row's `revision` advances only when a field actually changed**
  (`ON CONFLICT ... DO UPDATE ... WHERE existing IS DISTINCT FROM excluded`), so
  `/sync/delta` returns the rows that changed rather than everything touched.
- **`last_full_reload_revision` is never written.** That is what makes delta sync
  work across refreshes, and it is the single line most likely to be reintroduced
  by accident.
- **`bin/ingest` is deleted, not kept behind a flag.** Its only remaining use is
  "rebuild from scratch", which the sync does anyway against an empty database.
  Keeping it preserves a footgun for a case that no longer exists.
- **A binary run from CI, not an endpoint.** v1 has no write API
  ([ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md)), and adding one
  for this would be a far larger surface than a scheduled `bin/` tool.
- **Rows with a null `song_id` are skipped and counted.** None exist upstream
  today, but the column is nullable and `UNIQUE` permits many `NULL`s, so such a
  row would be re-inserted on every run and accumulate forever.

## Out of scope

Chart text. `charts` is populated separately by `seed_songs`, and this feature
does not read or write it — a song may exist in the catalog with no chart, and a
chart may outlive its song's disappearance from upstream.

Vendoring the upstream snapshot and the PR-based refresh flow
([`prod-data-and-infra`](../prod-data-and-infra/spec.md) 02 and 03). Those assume
truncate-and-reload and are reshaped by this feature; they are amended rather
than deleted, because the backup role of a vendored snapshot survives
independently of how rows are applied.

## Done when

The daily workflow runs unattended against production, a run with no upstream
changes bumps zero row revisions, a run with changes bumps exactly the rows that
changed, `GET /sync/delta?since=N` keeps working across refreshes instead of
answering 409, a truncated upstream response aborts the job with nothing written,
and `bin/ingest` no longer exists.
