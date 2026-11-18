# 10 — Browse

**What to build:** the *Browse* screen and its worked example: search, difficulty chips and
level range on the surface, everything else in the filter drawer, the result
line with My list and Draw, the grid only (list view removed), the pager.

**Blocked by:** 08.

**Status:** done

- [x] Every row of the worked-example table (the Modal row lands with 11)
- [x] Drawer from the right on desktop, from the bottom on phones; live filters, Reset, no Apply
- [x] Grid 2 / 3 / 4 columns at the two breakpoints
- [x] Empty states for no matches and an empty list
- ~~Filter state still round-trips through the URL query~~ — dropped: it never
  did. The filters live in component state; the browse page had no query
  sync to keep. Shareable filter links would be new work, not a regression.

## Comments

**2026-11-18 — built.** `BrowseView` is the surface: search (360px, full
width on phones), the Filters button, difficulty chips in the underline form
coloured from `difficulties[].color`, the level range as two 96px selects,
and the result line (count, My list with the bookmark count, Draw). The
drawer (`MvDrawer`, new in `components/ui/`) holds `BrowseFilters`: title
and artist exact match, artist, designer, category, version, region and its
override, type chips, internal level, BPM range. Toggles are filter chips.
Reset clears the drawer's filters and leaves the surface's alone. The list
view, the view toggle and `BrowseGrid`/`List`/`Pagination`/`ResultBar`/
`SearchBar` are gone.

Departures:

- **Cards open the old sheet modal by click** until 11 gives the song modal
  a URL: `MvCard` without `to` renders a button and emits `open`.
- **My list uses the bookmark icon**, not the worked example's star: the
  card's mark is a bookmark, and one concept keeps one icon.
- **24 cards a page**, not 22: 24 fills whole rows at 2, 3 and 4 columns.
- **A field can be a group.** `MvField group` labels a chip row or a range
  through `role="group"`; its controls carry their own names (`label` on
  `MvTextInput`/`MvSelect`), so two inputs never share the field's id.
- **The catalog's error state** is `MvError` with Retry; `loadData` now runs
  again from `ERROR`.

**2026-11-18 — review.** Applied: the drawer's focus handling moved into
`composables/useDialogFocus.ts` (the song modal shares it), and Tab or
Shift+Tab from the panel itself now lands inside rather than behind the
scrim. The bookmark mark keeps one name ("Bookmark") and lets
`aria-pressed` carry the state. BPM text that is not a number filters
nothing instead of emptying the list, and the BPM inputs ask for a decimal
keypad. `MvError` is framed in `--fg`, not the danger colour, as the spec
says. The Filters button declares `aria-haspopup="dialog"` and
`aria-expanded`. Kept: the level selects offer the same thresholds with
internal level on — they are numeric bounds, and `14+` as a lower bound
still reads right against an internal `14.7`. Open: the changed English copy
(`myListEmpty`, `filterResultEmpty`, `myList`, `drawRandom`,
`searchPlaceholder`) is stale in the eight other locales until 13.
