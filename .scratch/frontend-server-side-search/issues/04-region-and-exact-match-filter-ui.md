# 04 — Add region/override/exact-match/internal-level filter UI

**What to build:** `BrowseFilters.vue` gains UI for filter fields that already exist in the filter engine but have no control today: a Region select + "use region override" checkbox, exact-match checkboxes for title/artist, and a "use internal level" checkbox. `BrowseForm` type extended to match; new locale keys added across all 9 locale files (English text as placeholder is acceptable — no i18n-completeness check exists in this project).

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `BrowseForm` gains `region`, `useRegionOverride`, `matchExactTitle`, `matchExactArtist`, `useInternalLevel`; component gains `regionOptions` prop
- [ ] Region select + override checkbox added next to Version field; exact-match checkboxes next to Title/Artist; internal-level checkbox next to Level range
- [ ] `.mv-check` style added
- [ ] New locale keys (`exactMatch`, `useInternalLevel`, `useRegionOverride`) added to all 9 locale files
- [ ] `pnpm build && pnpm dev` — new controls render and toggle without console errors
