# 06 — Delete now-dead filterSheets

**What to build:** Cleanup once `BrowseView.vue` no longer calls `filterSheets` (ticket 05): delete the function from `filter.ts`, and `getRegionOverrideSheet` from `sheet.ts` if it's now unreferenced too.

**Blocked by:** 05 — rewire BrowseView.vue.

**Status:** done

- [ ] `grep -rn "filterSheets" apps/host/src` shows only the (about to be deleted) definition
- [ ] `filterSheets` deleted from `filter.ts`; `buildFilterOptions`/`buildEmptyFilters` kept (still used)
- [ ] `getRegionOverrideSheet` deleted from `sheet.ts` if no other callers remain (check via grep first)
- [ ] `pnpm build` passes
