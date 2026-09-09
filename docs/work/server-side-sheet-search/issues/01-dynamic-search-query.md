# 01 — queries.rs: dynamic search_sheets query

**What to build:** `SheetSearchParams` struct (mirrors the frontend's `Filters` type minus `superFilter`) and `search_sheets(pool, params) -> (Vec<(SongRow, SheetRow)>, total)` in `queries.rs`, built at runtime via `sqlx::QueryBuilder` since the filter set is optional/combinatorial (can't be expressed in a single `query_as!`). Handles `|`-delimited multi-category matching, region-override-aware level/designer columns (COALESCE), and pagination with a total count.

**Blocked by:** `server-catalog-typed-queries/issues/03-types-rs-response-shape-seam.md` (consumes `SongRow`/`SheetRow`/`RegionOverride`; `SongRow` gains a `#[derive(sqlx::FromRow)]` in this ticket).

**Status:** done

- [ ] `SheetSearchParams` struct with `Default` impl (page 1, pageSize 22)
- [ ] `search_sheets` builds dynamic WHERE via `QueryBuilder`: title/artist substring-or-exact, categories (`|`-split match), versions/types/difficulties (exact), level/BPM ranges, note designers, region include/exclude (`!` prefix), region-override COALESCE substitution for level/designer columns when `useRegionOverride` is set
- [ ] `#[sqlx::test]` coverage: title substring filter, level range filter, pagination + total count, pipe-delimited category match, region-override-before-level-filter ordering — all pass
- [ ] `SongRow` gains `#[derive(sqlx::FromRow)]`
