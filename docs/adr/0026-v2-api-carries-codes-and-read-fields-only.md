# ADR-0026 — The v2 API carries codes and read fields only; a sheet is addressed through its song

**Status:** accepted
**Date:** 2026-11-17

## Context

`api-rewrite` replaces `/api/v1` wholesale, on the `v2` branch, cutting over
with the redesigned frontend. Its spec settled identity (a minted UUID per
song) and addressing (`/api/songs/{id}/sheets/{type}/{difficulty}`) and left
the payload open until the frontend's data requirements existed.
[`screen-data-requirements.md`](../reference/screen-data-requirements.md)
now lists what every screen reads. Three questions remained, and one
disagreement: that document has the visualizer entered by `?sheet=<id>` and
bookmarks keyed by "the minted sheet id", while sheets have no minted id —
`sheets.id` is a bigserial nothing outside the database sees.

## Decision

1. **A sheet is identified by its song's id, its type and its difficulty
   code.** The URL is the spec's
   `/api/songs/{id}/sheets/{type}/{difficulty}`; the client's sheet key and the
   visualizer's link are built from the same three parts. No sheet id is
   minted.
2. **Payloads carry only fields a screen reads.** `songId` (the title-derived
   upstream key), `isNew`, `isSpecial`, `versions[].releaseDate`, the type and
   difficulty icon fields, `updateTime`, and the manifest's `updateTime`,
   `catalogHash` and `counts` are not sent. A field returns when a screen
   needs it, one at a time, by amending the contract.
3. **Difficulty is the `code` on the wire, everywhere.** A sheet's
   `difficulty` is `difficulties.code` (`master`, `協`), the same string its
   URL and a community chart carry; the reference table maps the code to its
   `name` and `color`.
4. **`/sync/delta` is not carried into v2.** Freshness is the manifest's
   revision plus the ETag'd Catalog.

## Rejected alternatives

- **A UUID v7 per sheet.** Shorter links and one opaque key, but a migration of
  `sheets.id`, `charts.sheet_id` and every `sheet_*` table, and two spellings of
  a sheet — by id, and by song, type and difficulty, which the sync still
  needs. The song id is already minted and a sheet's type and difficulty never
  change, so the composite is as stable as a minted id.
- **Keep the unread fields.** A superset payload spares a later screen a server
  change, but documents fields with no reader in a contract written from
  scratch; that is how the v1 shape accumulated them.
- **The bracketed `name` (`【協】`) as the sheet's difficulty.** Two spellings of
  one difficulty between payload and URL, and a strip-or-add rule on the
  client; `catalog-difficulties` split `code` from `name` to avoid exactly that.
- **Port `/sync/delta`.** It has no caller: the client probes the manifest and
  refetches the Catalog. Porting it would keep `test-foundation` 02's two
  deferred tests owed and need its payload redesigned on the new identity.

## Consequences

The Catalog snapshot test cannot be byte-compared with v1's; it becomes a
field-by-field mapping check, which the api-rewrite spec's tripwire asks for
anyway. `deleted_songs` and `deleted_sheets` are still written by the sync and
read by no endpoint once v1 is deleted. A screen that wants a dropped field
reopens the payload for that field only.
