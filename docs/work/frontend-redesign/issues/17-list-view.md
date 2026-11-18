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
