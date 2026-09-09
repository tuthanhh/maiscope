# 03 — useSheetSearch composable + Tauri search command

**What to build:** `apps/host/src/composables/useSheetSearch.ts` — wraps `GET /sheets/search` (debounced 300ms via `useDebounceFn`), decorates results via `decorateSheetFields`, dispatches through the Tauri `search_sheets` command when running native (same CORS-dodge pattern as the existing `load_chart_data`) or `fetch` in-browser.

**Blocked by:** 01 — split preprocessData (needs `decorateSheetFields`), `server-side-sheet-search/issues/02-search-endpoint-handler.md` (needs `GET /sheets/search` to exist).

**Status:** done

- [ ] `apps/host/src-tauri/src/search.rs`: `search_sheets` Tauri command, mirrors `data::load_chart_data`'s CORS-dodge pattern exactly
- [ ] Wired into `src-tauri/src/lib.rs`'s `invoke_handler`
- [ ] `useSheetSearch()` returns `{ results, total, loading, search }`, `search` debounced 300ms, decorates every result via `decorateSheetFields`
- [ ] `cd apps/host/src-tauri && cargo build` compiles
- [ ] `pnpm build` passes
