# 06 — `GET /api/sync/manifest`

**What to build:** the freshness probe, cut to what the client reads.
`stores/data.ts` compares `manifest.revision` with its cached revision and
reads nothing else.

**Blocked by:** 01.

**Status:** done

- [x] `GET /api/sync/manifest` → `{ "revision": n }`; `0` before the first sync
- [x] ETag and `304` from `Freshness`; `Cache-Control: no-cache`, as v1
- [x] No `/api/sync/delta` ([ADR-0026](../../../adr/0026-v2-api-carries-codes-and-read-fields-only.md))
- [x] Test: shape, the revision-0 sentinel on an empty `catalog_meta`, `304`
- [x] Contract: §3 gains the v2 manifest and says delta is not carried over

## Comments

**2026-11-17 — built.** One query (`Freshness::load`), so no snapshot is
needed. The manifest's ETag is the Catalog's, as in v1; a client that sends
it back gets a `304` until the revision moves.
