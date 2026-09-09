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

**Status:** todo

- [ ] `domain.rs` holds `Difficulty`, `ChartType`, `code()` impls
- [ ] One `routes/*.rs` per resource; each exposes a `Router<AppState>`
- [ ] `queries.rs` split to mirror `routes/`; SQL text unchanged
- [ ] Query-param DTOs (`ChartQuery`, `SheetSearchQuery`, `CatalogQuery`,
      `DeltaQuery`) live beside the handler that uses them
- [ ] `main.rs` under ~80 lines: config, tracing, pool, router, serve
- [ ] Tests move to `tests/` or per-module `#[cfg(test)]`, all still passing
- [ ] Route paths and the `/api/v1` nest are unchanged — no contract drift
