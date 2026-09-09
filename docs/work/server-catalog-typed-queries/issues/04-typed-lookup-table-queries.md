# 04 — queries.rs: typed lookup-table queries

**What to build:** `apps/server/src/queries.rs` (new file) gains typed, compile-time-checked query functions for the catalog's lookup tables: `fetch_categories`, `fetch_versions`, `fetch_types`, `fetch_difficulties`, `fetch_regions`, `fetch_update_time`. Each returns a `Vec<...Entry>` (or `String` for update time) via `sqlx::query_as!`, ordered by each table's `ordinal` column. Wired into `main.rs` via `mod queries;`.

**Blocked by:** 03 — types.rs response-shape seam (consumes `CategoryEntry`/`VersionEntry`/`TypeEntry`/`DifficultyEntry`/`RegionEntry`).

**Status:** done

- [ ] `fetch_categories`, `fetch_versions`, `fetch_types`, `fetch_difficulties`, `fetch_regions`, `fetch_update_time` implemented via `sqlx::query_as!`/`query_scalar!`, each ordered correctly
- [ ] `#[sqlx::test]` coverage: category ordering, version release-date formatting, update-time formatting — all pass
- [ ] `mod queries;` wired into `main.rs`
- [ ] `cargo build` compiles clean (unused-fn warnings for not-yet-called queries are expected)
