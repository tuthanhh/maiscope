# Database schema

Postgres schema for the maiscope global backend. Two concerns:

1. **Canonical** — the catalog served to clients. Mirrors the frontend `Data`
   shape (`apps/host/src/types/{Data,Song,Sheet}.ts`) so the client can rehydrate
   its frozen, prototype-linked object graph (`utils/data.ts:preprocessData`)
   unchanged. **Built now** — migration `20260624091738_canonical.sql`.
2. **Charts / assets / contributions** — chart storage + open contribution &
   moderation flow. **Later migrations** (see `docs/api-contract.md` §2, §5).

Cross-tier key everywhere: **`sheet_expr = songId|type|difficulty`**
(`utils/sheet.ts:computeSheetExpr`).

The server stores **raw fields only**. Derived fields the client computes
(`songNo`*, `imageUrl`, `imageUrlM`, `sheetExpr`*, `notePercents`,
`$canonicalSheet`) are not authoritative here. (*`song_no`/`sheet_expr` are
denormalized as a convenience/cache, but the client recomputes them.)

---

Single game (maimai) — `app/game.ts` removed the multi-game routing/registry, so
there is **no `games` table and no `game_code`**. Every table maps 1:1 to a piece
of `Data` (`apps/host/src/types/Data.ts`).

## Canonical tables (built)

### `catalog_meta`
Singleton (one row, pinned by a `CHECK (id)` on a boolean PK). Holds
`Data.updateTime` — the catalog freshness stamp and ETag source (contract §1).
The only top-level scalar in `Data`.
- `revision` — monotonic counter shared by every table that participates in
  sync. `bin/sync_catalog` draws `previous + 1` per run and stamps it onto the
  rows it actually changed; `apply_chart_revision` bumps it the same way for a
  single chart. `GET /sync/delta?since=N` compares row revisions against this
  one sequence, so a writer that invents its own counter breaks delta polling.
  **A run that changed nothing does not advance it** — this value feeds the
  `/catalog` ETag, so advancing it daily on a no-op would make every client
  refetch the whole payload. Writers must read it under the singleton's row
  lock (`bin/sync_catalog` takes it with its opening upsert) rather than with a
  bare `SELECT`, or a read-then-write can regress a concurrent bump.
- `last_full_reload_revision` — the revision of the last reload that broke row
  identity. A `since` older than this cannot be diffed, so `/sync/delta`
  answers `409 snapshot_required` (contract §3). **Nothing writes it any more.**
  `bin/sync_catalog` upserts by natural key and never deletes, so row identity
  survives every refresh; the column is frozen at whatever the retired
  `bin/ingest` left. That is exactly what makes delta sync work across daily
  refreshes.

Both feed the `ETag` on `GET /catalog` and `GET /sync/manifest`, which is
`sha256("{revision}:{updateTime}")`.

### Lookup tables — `categories`, `versions`, `types`, `difficulties`, `regions`
The `Data.categories/versions/types/difficulties/regions` arrays. The frontend
renders them as ordered lists and turns them into index maps (`stores/data.ts`:
`categoryIndexMap`, `versionMap`, `typeMap`, …). Array order is **significant**,
so each carries an `ordinal` instead of relying on insertion order. The natural
string (`category`/`version`/`type`/`difficulty`/`region`) is the PK.
- `categories` — song genre/category buckets.
- `versions` — game versions (`abbr`, `release_date`) for the version filter/sort.
- `types` — chart types (e.g. maimai std/DX). Carries `name`/`abbr` + raw
  `icon_url`/`icon_height` (client resolves the URL to absolute).
- `difficulties` — basic/advanced/expert/master/re:master. `name`, `color`,
  icon. The `Difficulty` enum in `main.rs` is the request-side mirror of this.
- `regions` — release regions (e.g. intl/jp) used by `sheet_regions` and
  `sheet_region_overrides`.

### `songs`
One row per song. Mirrors `Song.ts` raw fields (`song_id`, `category`, `title`,
`artist`, `bpm`, `image_name`, `version`, `release_date`, `is_new`, `is_locked`,
`comment`). Extras:
- `song_no` — derived display order (client also recomputes in `preprocessData`).
- `source_index` — position in the upstream `data.json`, so a sync is
  reproducible and stable ordering survives re-imports.
- `song_id UNIQUE` — natural identity (nullable per `Song.songId: string | null`).

### `sheets`
One row per playable chart slot of a song (a song has several sheets across
type×difficulty). Mirrors `Sheet.ts` raw fields (`type`, `difficulty`, `level`,
`level_value`, `internal_level`, `internal_level_value`, `note_designer`,
`is_special`). Key columns:
- `song_id_fk` — the real link to `songs.id` (source of truth).
- `sheet_expr` — denormalized cross-tier key, `UNIQUE`; the lookup key for
  `GET /sheets/{sheetExpr}` and charts.
- `source_index` — upstream ordering, as on `songs`.

> Note: this is **catalog metadata about a chart**, not the chart notes
> themselves. The actual simai/ma2 data lives in the `charts` table, written
> through `apply_chart_revision` (`server-restructure` issue 09) — see §5.

### `sheets` sub-tables (1-to-many off a sheet)
Normalized out of the `Sheet` object's `Record<...>` maps:
- **`sheet_note_counts`** `(sheet_id, key, value)` — the `noteCounts` map
  (`total`/`tap`/`hold`/`slide`/`touch`/`break`, …). `value` is **nullable**
  matching `Record<string, number | null>`. Client derives `notePercents` from
  these; the server does not store percentages.
- **`sheet_regions`** `(sheet_id, region, available)` — the `regions` map
  (`Record<string, boolean>`): is this sheet available in a given region.
- **`sheet_region_overrides`** `(sheet_id, region, …)` — the `regionOverrides`
  map. **Row existence = an override applies** in that region; non-null columns
  override the canonical sheet values (`level`, `level_value`, `internal_level`,
  `internal_level_value`, `note_designer`), null columns inherit.

> `bin/sync_catalog` reconciles all three of these for **every** sheet in the
> payload, not only sheets whose scalar columns changed — they move upstream
> independently (note counts backfilled after release, a region flipping to
> available). A sheet whose sub-tables alone changed still gets a fresh
> `revision`, so `/sync/delta` reports it. The per-sheet `DELETE`s that do the
> replacing are scoped to a single `sheet_id` and must stay that way.

### `deleted_songs` / `deleted_sheets`
Tombstones — `(song_id, revision)` and `(sheet_expr, revision)`. **Populated**,
as of `bin/sync_catalog`; they were empty placeholders under `bin/ingest`.

A row that disappears from the upstream payload is **not** deleted. Deleting a
song cascades through `sheets` into `charts` and destroys chart text, which
Postgres is the only copy of ([ADR-0001](../adr/0001-postgres-source-of-truth-for-charts.md)).
The row stays in `songs`/`sheets`, stays visible in `/catalog`, and gets one
tombstone row recording the revision it went missing at. `/sync/delta` serves
those under `tombstones` (contract §3). A row is logged **once**, not on every
subsequent sync — the PK is `(key, revision)`, so `ON CONFLICT DO NOTHING`
would still write a fresh entry under each new revision.

Accepted cost: the catalog drifts from upstream over time, with no way to tell
a current song from one dropped years ago. See `docs/work/catalog-sync/spec.md`
for the rejected alternatives.

---

### `charts`
The chart notes themselves — one canonical row per `(sheet_id, format)`,
enforced by `UNIQUE (sheet_id, format)`. `content` holds inline simai/ma2
text (`blob_url` is the alternative for large/binary payloads; only one is
used). `hash` is sha256 over `content` and is what makes a re-seed of
identical bytes a no-op. `sheet_expr` is denormalized off `sheets` for
lookup by the cross-tier key. `version` increments on each overwrite.

### `chart_revisions`
Append-only history — every applied revision adds a row (`chart_id`,
`content`, `hash`), enabling rollback and audit. Never updated in place.

> Both are written **only** through `chart_revision::apply_chart_revision`
> (`server-restructure` issue 09), which upserts `charts`, appends here, and
> bumps `catalog_meta.revision` in one transaction. `bin/seed_songs` calls it
> today; phase 2's contribution-approve handler will call the same function.
> Contributor/contribution FKs are still nullable placeholders until §5 lands.

## Relationships

```
catalog_meta   (singleton: update_time)

categories / versions / types / difficulties / regions   (standalone lookups, ordinal)

songs ─< sheets ─┬─< sheet_note_counts
                 ├─< sheet_regions
                 └─< sheet_region_overrides
```
Lookups are standalone (no FK from songs/sheets — `Song.category` etc. are plain
strings the client cross-references, matching the frontend's loose coupling). FKs
on the song→sheet→sub-table chain are `ON DELETE CASCADE`: drop a song → its
sheets + sub-rows vanish — **and so does the chart text hanging off those
sheets**, since `charts.sheet_id` cascades too. No code path deletes from
`songs`, `sheets` or `charts`; that is a structural guarantee, not a
convention. See `deleted_songs`/`deleted_sheets` above.

## Planned tables (not yet migrated)

| Table | Purpose | Contract |
|-------|---------|----------|
| `users` | GitHub-OAuth identities + role | §4 |
| `contributions` | open submission queue (`payload` JSONB, `status`) → merge on approve | §5 |
| `audit_log` | moderator/admin action trail | §5 |

## Conventions

- `id BIGSERIAL` surrogate PK on `songs`/`sheets`; natural composite PKs on
  lookups and sub-tables.
- Timestamps `TIMESTAMPTZ DEFAULT now()`; dates `DATE`.
- Numerics `DOUBLE PRECISION` (`bpm`, `*_value`); counts `INTEGER`.
- Migrations are **reversible**: each is a `.up.sql` / `.down.sql` pair. `0001`
  down drops all canonical tables (`IF EXISTS` + `CASCADE`, idempotent). sqlx
  tracks applied versions in `_sqlx_migrations` and never re-runs an applied up.
  - apply: `sqlx migrate run` · roll back last: `sqlx migrate revert`
