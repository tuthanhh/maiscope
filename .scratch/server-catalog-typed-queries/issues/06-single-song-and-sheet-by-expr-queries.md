# 06 — queries.rs: single-song and single-sheet-by-expr queries

**What to build:** `fetch_song_by_song_id(pool, song_id) -> Option<SongRow>` and `fetch_sheet_by_expr(pool, sheet_expr) -> Option<(SongRow, SheetRow)>` in `queries.rs`, both typed via `sqlx::query_as!`, backing the eventual `GET /songs/{id}` and `GET /sheets/{sheetExpr}` rewrites.

**Blocked by:** 03 — types.rs response-shape seam (consumes `SongRow`/`SheetRow`).

**Status:** done

- [ ] `fetch_song_by_song_id` returns `None` for unknown id, `Some(SongRow)` with correct fields for a known one
- [ ] `fetch_sheet_by_expr` returns `None` for unknown expr, `Some((SongRow, SheetRow))` correctly joined for a known one
- [ ] `#[sqlx::test]` coverage passes for both: not-found case, found case
