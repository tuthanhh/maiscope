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

**Status:** done

- [x] `GET /catalog` emits `ETag` derived from `catalog_hash`
- [x] `If-None-Match` match → `304` with no body
- [x] `Cache-Control: public, max-age=3600` on `/catalog` — see Comments for
      the reasoning and why `/sync/manifest` gets a different value
- [x] `GET /sync/manifest` also emits an `ETag` (it is the cheap freshness probe)
- [x] `Vary: Accept-Encoding` set so compressed and uncompressed responses do not
      poison one another in shared caches — already added automatically by
      `tower_http`'s `CompressionLayer` (ticket 06); verified live, no code
      needed
- [x] Test: two requests, second with `If-None-Match`, asserts `304` and empty body
- [x] Test: catalog change bumps the ETag

## Comments

**`Cache-Control` values, and why they differ by endpoint:** `/catalog`
(`public, max-age=3600`) only changes on a `bin/ingest` run, not
continuously, so a 1-hour window trades a small staleness risk for the best
case this pairing enables — the browser skips the network entirely within
that window, not just downgrading to a cheap `304`. `/sync/manifest`
(`no-cache`) is deliberately different: it's the sync tier's freshness
*probe* (contract §3) — if the browser trusted a locally cached manifest
without asking the server, polling for new data would silently stop
working. `no-cache` still gets the `304` win via the `ETag` above it, just
without ever skipping the round-trip.

**Real bug found and fixed while wiring this up:** `sync_manifest` took a
`headers` parameter but never read it — the `If-None-Match` check was
declared but not implemented, so manifest never actually returned `304`
despite emitting an `ETag`. Separately, its `ETag` header value wasn't
quoted (`catalog`'s is, per RFC 7232 — `ETag` values must be a quoted
string). Since browsers always send back whatever the server gave them,
an unquoted emit would never have matched a client's `If-None-Match` even
after adding the comparison. Both fixed together; the JSON body's own
`catalogHash` field is intentionally still the raw unquoted hash — that's
a data field (contract §3), not an HTTP header.

## Follow-up review (correction)

**The `304`s shipped bare.** Both handlers returned
`StatusCode::NOT_MODIFIED.into_response()` with no headers at all. RFC 7232
§4.1 requires a `304` to repeat the validators the `200` would have sent,
because that is how the client refreshes the freshness of the copy it
already holds — without a `Cache-Control` on the `304`, the stored catalog
expires again immediately and the very next visit revalidates too, which
gives back most of the `max-age=3600` this ticket was for.

Both now send `ETag` + `Cache-Control` on the `304`, pinned by
`catalog_304_repeats_the_etag_and_cache_control` and
`manifest_304_repeats_the_etag_and_revalidates`.

The duplicated preamble the two handlers shared — same `catalog_meta`
query, same `format!("\"{}\"")`, same `If-None-Match` compare — moved to
`routes/caching.rs` (`Freshness::load` / `is_current_for` / `not_modified`,
plus the two `Cache-Control` constants). `catalog_hash` moved there from
`routes/catalog.rs`; `routes/sync.rs` no longer reaches across into the
catalog module for it.
