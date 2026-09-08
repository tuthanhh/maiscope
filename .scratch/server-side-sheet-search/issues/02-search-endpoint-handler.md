# 02 — GET /sheets/search handler

**What to build:** New route `GET /sheets/search`, query-param struct mirroring `Filters` (camelCase→snake_case), calling `queries::search_sheets` and returning `{ sheets: Sheet[], total }`.

**Blocked by:** 01 — dynamic search query.

**Status:** done

- [ ] `SheetSearchResponse { sheets: Vec<Sheet>, total: i64 }` added to `types.rs`
- [ ] `GET /sheets/search` handler parses all query params, clamps `pageSize` to `[1, 100]`, calls `queries::search_sheets`, decorates via `into_meta()`
- [ ] Route wired without colliding with `/sheets/{sheet}`
- [ ] Handler-level integration test: paginated response shape correct
- [ ] Manual smoke test: `curl "/api/v1/sheets/search?title=...&page=1&pageSize=10"` returns valid `{ sheets, total }`
