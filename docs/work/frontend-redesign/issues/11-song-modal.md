# 11 — Song modal with a URL

**What to build:** the *Song modal*: card tap opens it,
`?song=<id>` in the URL so it can be shared and Back closes it, information
first, difficulty chips in the underline form, bookmark and play as text
buttons. The song page and the old sheet dialog are deleted.

**Blocked by:** 08, 10.

**Status:** done

- [x] Opening a `?song=<id>` link without the Catalog loaded fetches `/api/songs/{id}` (screen-data-requirements: a shared link must not cost the snapshot)
- [x] Full-height sheet from the bottom on phones
- [x] No video search, no lock or globe icons
- [x] `pages/song.vue` and `MvSheetDialog.vue` removed; their routes redirect to the modal

## Comments

**2026-11-18 — built.** The modal is URL state on the Songs route:
`#/songs?song=<id>&type=<type>&difficulty=<code>` (`composables/useSongModal.ts`).
A card is a link to it, so a card push opens it and Back closes it; the
type and difficulty chips replace the entry rather than stacking one per
tap. Closing goes back when the entry before is the same page without the
modal, and replaces otherwise (a shared link has nothing to go back to).
`SongModal.vue` takes the song from the Catalog when it is loaded and
fetches `/api/songs/{id}` otherwise; the global loading overlay stays off
while a song link is open so that fetch actually paints first. 400 and 404
read as "Song not found"; any other failure offers Retry. `/song?id=`
redirects to `/songs?song=`. Removed: `pages/song.vue`, `MvSheetDialog.vue`,
`useSheetDialog.ts`, `useItemDrawer.ts`.

New base pieces: `MvModal` (centred, full-height sheet on phones) and the
`useDialogFocus` composable it shares with the drawer.

Departures:

- **A type row** appears above the difficulty chips when a song has more
  than one chart type (STD and DX): the design's single row would show two
  BASICs. Type chips are filter chips (inverse when selected).
- **Play is disabled without a chart**, with "No chart yet" beside it,
  rather than opening an empty visualizer.
- **Colour needs the Catalog.** On a shared link before it loads, the
  difficulty underline and the level fall back to `--fg`; 12 fetches
  `/api/reference` at boot, which fixes this.
- **The song comment is gone** from the modal; the design does not list it.

**2026-11-18 — review.** Applied: a bookmark toggled on a shared link
before the Catalog installed was added a second time beside its persisted
key, and could then never be removed. The store now owns bookmarks
(`hasBookmark`, `toggleBookmark`): a still-unresolved key counts as
bookmarked and toggles off where it is, `install` de-duplicates, and every
change is persisted whatever the Catalog's state. The scroll lock is
reference-counted across dialogs, so the always-mounted drawer no longer
clears the lock of a modal opened at mount. The loading overlay is held off
only for a song link on Songs, and the browse behind it says "Loading data…"
rather than "No sheets match". Closing compares normalised paths. A link
naming no sheet of the song is corrected to the sheet shown; a song without
sheets says so instead of loading forever; the modal's fallback name is
"Song"; the error block carries its sentence; disabled Play is described by
"No chart yet". Kept: the chip groups use `aria-pressed` — only one chip is
ever pressed, so it reads as the selection.
