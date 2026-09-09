# 05 — Rewire BrowseView.vue onto server-side search

**What to build:** `BrowseView.vue` swaps its client-side `computed(() => filterSheets(...))` + local `PAGE_SIZE` slice for `useSheetSearch` + the server's `total`/`page`. Two real, documented behavior narrowings accepted as trade-offs (not silently shipped, not fixed here): "My List Only" now filters within the current page only (not the full filtered set before paginating), and `drawRandom` now only picks from the currently loaded page. Both flagged with inline comments/TODOs, not hidden.

**Blocked by:** 02 — remove superFilter, 03 — useSheetSearch composable, 04 — region/exact-match filter UI.

**Status:** done

- [ ] `emptyForm()` extended with the 5 new fields; `regionOptions` computed and passed to `BrowseFilters`
- [ ] `filters` computed maps every `BrowseForm` field (including the 5 new ones) into `Filters`
- [ ] `results`/pagination driven by `useSheetSearch` — `watch(filters, ...)` triggers a debounced server search and resets to page 1; `watch(currentPage, ...)` re-searches
- [ ] `BrowseResultBar`/`BrowsePagination` use `searchTotal`, not a client-computed length
- [ ] `drawRandom`'s reduced scope (current-page-only) flagged with an inline `TODO` comment, not silently fixed
- [ ] `pnpm build` passes
- [ ] Manual smoke test: first-page load fires a `/sheets/search?...page=1` request; debounced title search; chip toggles reset to page 1; region+override params appear in the request; pagination to page 2 fires a new request; "My List Only" with an out-of-page bookmark does not appear (documented trade-off, not a regression)
