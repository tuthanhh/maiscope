# 08 — Rewire GET /songs/{id} and GET /sheets/{sheetExpr}, final cleanup

**What to build:** `get_song` and `get_sheet` handlers in `main.rs` stop reading `v_song`/`v_sheet` and instead use `queries::fetch_song_by_song_id` + `queries::fetch_sheets_for_song` (for `get_song`) and `queries::fetch_sheet_by_expr` (for `get_sheet`), returning typed `Json<Song>`/`Json<types::Sheet>`. This is the last ticket in the typed-query migration — after this, nothing in the codebase queries the dropped JSON views.

**Blocked by:** 05 — typed sheet queries, 06 — single-song/sheet-by-expr queries, 02 — drop JSON views migration.

**Status:** done

- [ ] `get_song` returns 404 for unknown id, typed `Song` (with nested sheets, `hasChart` present) for a known one
- [ ] `get_sheet` returns 404 for unknown sheetExpr, typed `Sheet` (song fields flattened in) for a known one
- [ ] `cargo test --lib` — all tests pass across this whole feature (lookup, sheet, song/sheet-by-expr queries, catalog handler, get_song/get_sheet handlers)
- [ ] `cargo build` — no unused-import warnings for `Value`/`json`/the new types
- [ ] Manual smoke test: `/catalog`, `/songs/{id}`, `/sheets/{sheetExpr}`, and a 404 case for an unknown id all behave correctly
- [ ] `grep -n "hasAudio\|v_song\|v_sheet" apps/server/docs/api-contract.md apps/server/docs/schema.md apps/server/src/main.rs apps/server/src/queries.rs` returns no output
