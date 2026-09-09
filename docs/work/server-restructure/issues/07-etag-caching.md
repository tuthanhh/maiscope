# 07 — Server: `ETag` / `Cache-Control` / `304` on catalog and manifest

**What to build:** Contract §1 already specifies `ETag`, `Cache-Control` and
`If-None-Match` → `304` for `GET /catalog`, and `catalog_hash(revision, update_time)`
already exists at `main.rs:209`. Wire it up.

This is the highest-leverage piece of free-tier protection: it turns a repeat
visitor from 4.7MB into a ~200-byte `304`. It is also the v1.0 client freshness
strategy — the IndexedDB blob cache (`web-delivery` issue 02 / v1.1) layers on top of it, not
instead of it.

**Blocked by:** 06 (interacts with compression — ETag must be stable across
encodings or vary correctly)

**Status:** todo

- [ ] `GET /catalog` emits `ETag` derived from `catalog_hash`
- [ ] `If-None-Match` match → `304` with no body
- [ ] `Cache-Control: public, max-age=…` chosen and justified in this ticket
- [ ] `GET /sync/manifest` also emits an `ETag` (it is the cheap freshness probe)
- [ ] `Vary: Accept-Encoding` set so compressed and uncompressed responses do not
      poison one another in shared caches
- [ ] Test: two requests, second with `If-None-Match`, asserts `304` and empty body
- [ ] Test: catalog change bumps the ETag
