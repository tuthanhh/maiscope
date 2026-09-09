# 05 — queries.rs: region-filterable sheet queries

**What to build:** Two typed query functions in `queries.rs`: `fetch_all_sheets(pool, region: Option<&str>) -> HashMap<i64, Vec<SheetRow>>` (every sheet grouped by parent song, optionally region-filtered via `EXISTS`/`sheet_regions`) and `fetch_sheets_for_song(pool, song_pk) -> Vec<SheetRow>` (unfiltered, for `GET /songs/{id}`). Both compute `has_chart` via an `EXISTS` subquery against `charts`, matching the old view's logic exactly.

**Blocked by:** 03 — types.rs response-shape seam (consumes `SheetRow`).

**Status:** done

- [ ] `fetch_all_sheets` groups correctly by `song_id_fk`, region filter keeps songs with zero matching sheets absent from the map (caller's job to fall back to empty vec)
- [ ] `fetch_sheets_for_song` ignores region entirely, returns all sheets for one song ordered by `source_index`
- [ ] Both compute `has_chart` per sheet correctly (test: sheet with a `charts` row → true, sheet without → false)
- [ ] `#[sqlx::test]` coverage passes: grouping, region filter (empty-vec case), unfiltered fetch, has_chart correctness
- [ ] `SheetRow` field declaration order matches every SELECT list exactly
