# 04 — Server: split `main.rs` and `queries.rs` into per-resource modules

**What to build:** `main.rs` is 614 lines holding bootstrap, routing, domain
enums (`Difficulty`, `ChartType`), five query-param DTOs, error helpers, eight
handlers and the test module. `queries.rs` is 866 lines of every query for every
resource in one flat file.

Split by resource. **Move code, do not rewrite it** — handler bodies and SQL stay
byte-identical wherever possible, so the existing tests remain a real safety net.

Target layout:

```
src/
  main.rs        bootstrap + router assembly only
  config.rs      (01)
  error.rs       (02)
  state.rs       (03)
  domain.rs      Difficulty, ChartType and their code() impls
  types.rs       response + row structs (unchanged)
  routes/
    mod.rs       router assembly per resource
    catalog.rs   songs.rs  sheets.rs  charts.rs  sync.rs  health.rs
  queries/
    mod.rs       catalog.rs  songs.rs  sheets.rs  charts.rs  sync.rs
```

**Blocked by:** 02, 03

**Status:** done

- [x] `domain.rs` holds `Difficulty`, `ChartType`, `code()` impls
- [x] One `routes/*.rs` per resource; each exposes a `Router<AppState>`
- [x] `queries.rs` split to mirror `routes/`; SQL text unchanged
- [x] Query-param DTOs (`ChartQuery`, `SheetSearchQuery`, `CatalogQuery`,
      `DeltaQuery`) live beside the handler that uses them
- [x] `main.rs` under ~80 lines: config, tracing, pool, router, serve (45 lines;
      tracing itself is ticket 05, not yet added)
- [x] Tests move to per-module `#[cfg(test)]`, all still passing (38/38)
- [x] Route paths and the `/api/v1` nest are unchanged — no contract drift
      (verified live: healthcheck, 404 song, 409 snapshot_required, sync/manifest,
      CORS headers all unchanged)

## Comments

`queries/charts.rs` and `queries/sync.rs` from the target layout don't exist.
Neither `get_chart` nor `sync_manifest`/`sync_delta` had their SQL in a separate
function before this ticket — it was always inline in the handler. Extracting it
now would be rewriting, not moving, which the ticket explicitly rules out
(`sync_delta` in particular has a for-loop threading five separate queries
through control flow; splitting that apart safely is its own piece of work, not
a mechanical file move). Left inline in `routes/charts.rs` and `routes/sync.rs`
exactly as they were in `main.rs`.

`catalog_hash` is used by both `routes::catalog` (the ETag) and `routes::sync`
(`sync_manifest`'s `catalogHash` field). Kept one `pub(crate)` copy in
`routes/catalog.rs`, imported by `routes/sync.rs` — not duplicated.

This ticket's actual starting point was a half-finished, already-broken split
(not the clean `main.rs`/`queries.rs` the ticket describes): handler bodies had
been copy-pasted into the wrong tree (`queries/catalog.rs` held the `catalog`
*handler*, not a query; `queries/sheets.rs` held `get_sheet`), `domain.rs`
existed but wasn't `pub` or declared as a module, `DeltaQuery` was duplicated in
two files, and `routes/mod.rs` didn't compile (bare `Router`, no imports,
referenced an undefined `state`). Rebuilt from the last-committed `main.rs`
(`3fe71dd`) instead of patching the half-done state, so handler bodies and SQL
are verified byte-identical to that commit.
