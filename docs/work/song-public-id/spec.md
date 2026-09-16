# Spec — Stable public song ids

**Status:** planned
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Make every song's primary key a stable, ASCII, never-derived-from-text
identifier, so the API can address one without percent-encoding a Japanese
title.

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
| 01 | [`songs.id` becomes a UUID](issues/01-mint-and-backfill.md) | 1845 existing rows, and `sheets.song_id_fk` with them |
| 02 | [`catalog_sync` mints for new songs](issues/02-sync-mints-new-songs.md) | Blocked by 01 |

## Decisions that constrain this work

- **Minted, not derived.** Not a slug, not a hash of the title, not
  `image_name`. Anything derived from text inherits the instability being fixed;
  `image_name` is stable but not unique — 180 non-UTAGE songs share a jacket.
- **The mint is UUID v7 in a native `uuid` column**, canonical hyphenated in
  URLs. Reasoning in issue 01's comments. The same scheme is reused for
  `community_songs` and `community_charts` — also as their primary keys — so
  both libraries address alike and neither carries two ids for one row.
- **It *is* `songs.id`.** Decided 2026-09-16, reversing an earlier decision to
  keep a bigserial primary key beside a separate `public_id` column. One
  identifier, not two: the primary key becomes `UUID` and the bigserial is gone.
  `sheets.song_id_fk` widens with it — the only column in the schema that
  references `songs(id)`.

  What that reversal costs, stated plainly because it was the original reason
  for keeping them apart: the primary key is now identity-bearing. Any operation
  that re-inserts songs — an identity-breaking full reload, the thing
  `catalog_meta.last_full_reload_revision` exists to signal — **must reuse each
  song's existing id by matching on `song_id`**, or every URL in the system
  changes at once. With two columns that hazard sat on a column nothing linked
  to. Now it sits on the primary key, so it has to be a tested property rather
  than a convention.
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

Every song's primary key is a UUID, a newly synced song gets one automatically,
no bigserial remains on `songs`, and no URL in the system contains a character
needing percent-encoding.

`song_id` remains the sync's conflict key, so this feature does not fix renames —
it only stops a rename from breaking every URL. Detection of renames is
[`sync-rename-alarm`](../sync-rename-alarm/spec.md).
