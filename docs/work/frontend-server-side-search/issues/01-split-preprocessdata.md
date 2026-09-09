# 01 — Split preprocessData into buildCatalog + freezeCatalog

**What to build:** `apps/host/src/utils/data.ts`'s fused mutate+freeze `preprocessData` splits into `buildCatalog` (field computation, no freezing) + `freezeCatalog` (separate freeze pass) + `preprocessData` (now a 2-line orchestrator, unchanged public signature). Also factors out `decorateSheetFields` (imageUrl/imageUrlM/sheetExpr/notePercents/`$canonicalSheet`) so the new search flow can decorate flat `Sheet[]` results without duplicating logic.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `decorateSheetFields(sheet, dataSourceUrl)` exported, mutates a flat Sheet in place, no `validateNoteCounts` call (catalog-build-only warning)
- [ ] `buildCatalog(data, dataSourceUrl, gameCode)` exported — everything `preprocessData` did minus `Object.freeze` calls
- [ ] `freezeCatalog(data)` exported — every freeze call, same relative order as before
- [ ] `preprocessData` unchanged signature, now just calls `buildCatalog` then `freezeCatalog`
- [ ] `pnpm build` (vue-tsc --noEmit) passes
- [ ] Manual smoke test: browse page renders identically (covers, levels, sheetExpr-dependent visualizer deep-link)
