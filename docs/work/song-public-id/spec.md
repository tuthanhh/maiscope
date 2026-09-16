# Spec — Stable public song ids

**Status:** planned
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Mint a stable, ASCII, never-derived-from-text identifier for every song, so the
API can address one without percent-encoding a Japanese title.

## Problem

`song_id` is the title. 1761 of 1845 songs have `song_id = title` exactly; the
rest are UTAGE variants carrying a label prefix. 1638 are not URL-safe, 1181 are
non-ASCII, and five contain `/`.

That makes it a poor URL component, but the deeper problem is that it is not
stable. `catalog_sync` upserts `ON CONFLICT (song_id)`, so an upstream title
correction presents an id the table has never seen: insert, not update. The old
row fails the vanish sweep and is tombstoned, and the seeded chart text stays
attached to the dead `sheet_expr` while the new row reports `hasChart: false`.

Every bookmark, cache entry and client-side index keyed on the old value points
at a row nothing will ever update again.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Mint and backfill `public_id`](issues/01-mint-and-backfill.md) | 1845 existing rows |
| 02 | [`catalog_sync` mints for new songs](issues/02-sync-mints-new-songs.md) | Blocked by 01 |

## Decisions that constrain this work

- **Minted, not derived.** Not a slug, not a hash of the title, not
  `image_name`. Anything derived from text inherits the instability being fixed;
  `image_name` is stable but not unique — 180 non-UTAGE songs share a jacket.
- **Distinct from `songs.id`.** The bigserial is a storage detail with no
  guarantee across an identity-breaking reload, which is exactly what
  `catalog_meta.last_full_reload_revision` exists to signal. `public_id` is
  stable because it is persisted and never recomputed.
- **`song_id` is not removed.** It stays as upstream's natural key and remains
  the sync's conflict target. It simply stops being the thing URLs and clients
  are built on.
- **Renames are not chased.** See [`api-rewrite`](../api-rewrite/spec.md) — a
  heuristic rematch was rejected on measured collision risk.

## Out of scope

`sheet_expr`. It stays as the denormalized storage key and the frontend's
identity key for indexing; only its role as a URL component goes away, and that
is the API rewrite's business.

Changing how the frontend indexes. That moves with the rewrite.

## Done when

Every song has a `public_id`, it is `UNIQUE NOT NULL`, a newly synced song gets
one automatically, and none of them contains a character needing percent-encoding.
