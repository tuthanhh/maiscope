# 02 — Client identity off `sheetExpr`

**What to build:** the frontend indexes on the minted song id, not on a
title-derived composite key.

`utils/sheet.ts:computeSheetExpr` builds `${songId}|${type}|${difficulty}`, and
`songId` is the title. Everything downstream — `$canonicalSheet`, the selected
sheets store, the sheet dialog, URL construction — inherits that.

**Blocked by:** [`song-public-id`](../../song-public-id/spec.md),
[`api-rewrite`](../../api-rewrite/spec.md).

**Status:** done

- [x] The minted song id (plus type and difficulty code) replaces `sheetExpr` as the identity used for lookup and links
- [x] `computeSheetExpr` either stops existing or is confined to whatever still
      genuinely needs the composite form
- [x] Persisted client state keyed on the old value (selected sheets, any
      localStorage) migrates or is discarded deliberately — silently dropping a
      user's selection is the failure mode
- [x] CLAUDE.md's "cross-tier key" convention is rewritten to match

## Comments

**2026-11-17 — built, together with the data layer's switch to `/api`.** The
minted id exists only in the v2 payload, so this ticket could not change the
key without changing the source:

- **Types.** `Song` has `id`; `Sheet` has an explicit `song` and a `key`
  (`${song.id}/${type}/${code}`, `utils/data.ts:sheetKey`) — ADR-0026's
  address. The prototype chain, `$canonicalSheet`, `songNo`, `imageUrlM`,
  `notePercents` and `computeSheetExpr` are gone. `preprocessData` stays,
  reduced to one stated job: point sheets at their songs, key them, derive
  the cover URL (the asset host is client configuration), freeze the graph.
- **Region overrides** are plain value maps; `utils/sheet.ts:regionalValues`
  reads a sheet's level and designer in a region, where v1 swapped in an
  override object that inherited the rest through its prototype.
- **Store.** `/api/sync/manifest` then `/api/catalog`; difficulties keyed by
  `code`. The IndexedDB cache moved to a new database named for the payload
  shape (`maiscope-catalog-v2`) and v1's is deleted — `frontend-redesign` 04's
  cache-versioning item, done here because it is the same change.
- **Bookmarks** were in memory only before, so there was nothing to migrate.
  They now persist per browser as sheet keys (`mv:bookmarks`), re-resolved
  against every Catalog install; a key whose sheet is gone drops out.
- **Visualizer deep link** is `?song=<id>&type=&difficulty=<code>`, fetching
  `/api/songs/…` for the card and the chart. v1's `?songId=<title>` links no
  longer resolve: a title is not an address.
- `useSheetSearch` deleted (no caller; endpoint gone, ADR-0021). Unused icon
  helpers deleted (icon fields are no longer sent). `deploy-web.yml` builds
  against `/api`.

Checked in headless Chromium against the local API: browse lists 7481
sheets, the MASTER chip filters to 1781, the dialog shows the song's fields,
a bookmark persists as a key across a reload, only the v2 IndexedDB
database remains, and the dialog's play link opens the visualizer with the
song's UUID and its card. The consumers were migrated mechanically
(`sheet.title` → `sheet.song.title`); restyling and the song modal are
ticket 04.
