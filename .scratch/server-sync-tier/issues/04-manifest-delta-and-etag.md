# 04 — GET /sync/manifest, GET /sync/delta, and GET /catalog's ETag

**What to build:** `GET /sync/manifest` (cheap freshness probe: updateTime, revision, catalogHash, row counts). `GET /sync/delta?since=` (rows changed since a revision, `409 snapshot_required` when `since` predates `last_full_reload_revision` — precisely defined against `ingest`'s TRUNCATE behavior, not a vague heuristic; includes songs whose sheets changed even if the song itself didn't, via a `LEFT JOIN`; `charts: []` and tombstones always-empty are explicit, documented scope cuts). `GET /catalog` gains `ETag`/`If-None-Match` → `304` support, sharing the same `catalogHash` concept.

**Blocked by:** 01 — revision tracking migration, 02 — ingest stamps revision, `server-catalog-typed-queries/issues/08-rewire-song-and-sheet-endpoints.md` (reuses `SongRow`/`NestedSheet`/`Song`/`fetch_sheets_for_song`).

**Status:** done

- [ ] `catalog_hash(revision, update_time)` shared helper (SHA-256)
- [ ] `GET /sync/manifest` returns `updateTime`, `revision`, `catalogHash`, `counts: { songs, sheets, charts }`
- [ ] `GET /sync/delta?since=`: `409 snapshot_required` when `since < last_full_reload_revision`; otherwise returns changed songs (song OR its sheets changed, via `LEFT JOIN ... WHERE so.revision > $1 OR s.revision > $1`), current `revision`, `charts: []`, and tombstones from `deleted_songs`/`deleted_sheets`
- [ ] `GET /catalog` returns `ETag` header; a matching `If-None-Match` request returns `304` with no body
- [ ] Manual smoke test: manifest, delta with `since=0`, delta with an out-of-range `since` (expect 409), catalog ETag round-trip (expect 304 on repeat with the same ETag)
