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
- `source_index` — position in the upstream `data.json`, so ingest is
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
> themselves. The actual simai/ma2 data lives in the future `charts` table.

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

---

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
sheets + sub-rows vanish.

## Planned tables (not yet migrated)

| Table | Purpose | Contract |
|-------|---------|----------|
| `chart_revisions` | append-only chart history → rollback/audit | §5 |
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
