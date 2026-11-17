# 06 — `GET /api/sync/manifest`

**What to build:** the freshness probe, cut to what the client reads.
`stores/data.ts` compares `manifest.revision` with its cached revision and
reads nothing else.

**Blocked by:** 01.

**Status:** todo

- [ ] `GET /api/sync/manifest` → `{ "revision": n }`; `0` before the first sync
- [ ] ETag and `304` from `Freshness`; `Cache-Control: no-cache`, as v1
- [ ] No `/api/sync/delta` ([ADR-0026](../../../adr/0026-v2-api-carries-codes-and-read-fields-only.md))
- [ ] Test: shape, the revision-0 sentinel on an empty `catalog_meta`, `304`
- [ ] Contract: §3 gains the v2 manifest and says delta is not carried over
