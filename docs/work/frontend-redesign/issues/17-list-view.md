# 17 — List view

**What to build:** the grid/list toggle on Songs again. Ticket 10 dropped
the list view by the design decision "grid only"; the maintainer reversed
it on 2026-11-18.

**Blocked by:** 10.

**Status:** done

- [x] A toggle in the result line (two icon buttons, the active one
      pressed); the choice is remembered per browser
- [x] List rows: jacket thumbnail, title (display face), artist, type and
      difficulty, level pill, bookmark mark; the whole row opens the song
      modal, as a card does; 44px minimum row height
- [x] Phones: the same rows, artist and type under the title
- [x] Paging, empty states and My list behave as in the grid
- [x] design-direction's "Browse" and decisions table record the reversal

## Comments

**2026-11-18 — built.** `BrowseList` renders the same paginated sheets as
the grid; `stores/browse.ts` holds `view`, read from and written to
`localStorage` (`mv:browse-view`) with every access guarded, so private
mode falls back to the grid. The toggle is two icon buttons in a labelled
group, the pressed one on `--bg-panel` — not the inverse fill, which the
design keeps for one strong state per screen. The `grid` and `list` icons
come back. Rows are 56px tall; the bookmark is a 44px target.

**2026-11-18 — review.** Applied: an edit had pasted the toggle's styles
into both grid media queries — removed. A row's and a card's accessible
name now starts with what they show (title, artist), so the name contains
the visible text (WCAG 2.5.3); the row's jacket is hidden from assistive
tech so it adds nothing to the name. Each bookmark button names its song.
`isBookmarked` is one stable function rather than a new one per render.
Kept: rows are 56px — the 48px jacket sets that; the design's 44px is the
table row's, for lists without artwork.
