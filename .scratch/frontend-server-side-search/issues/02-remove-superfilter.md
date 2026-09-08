# 02 — Remove superFilter

**What to build:** Cuts the never-UI-wired `superFilter` feature (arbitrary eval'd JS filter) entirely: `Filters` type, `parseSuperFilter`, the `superFilter` block in `filterSheets`, and the `superFilter` locale key across all 9 locale files.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `superFilter` removed from the `Filters` type and `buildEmptyFilters()`
- [ ] `parseSuperFilter` function and the `superFilter` block in `filterSheets` deleted
- [ ] `superFilter` locale key removed from all 9 locale files (`en.yaml` + 8 others)
- [ ] `grep -rn "superFilter\|parseSuperFilter" apps/host/src` returns no output
- [ ] `pnpm build` passes
