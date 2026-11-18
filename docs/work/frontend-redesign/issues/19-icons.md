# 19 — Icon set, app icon refresh, dark logo

**What to build:** the maintainer's ask of 2026-11-18, "all of the above":
the in-app UI icons from one maintained set instead of the hand-drawn
paths ported from the prototype; app and browser icons that installed
copies actually pick up; a dark-theme variant of the logo.

**Blocked by:** 15.

**Status:** done

- [x] Every `MvIcon` name drawn from Lucide (ISC licence), same 24px grid
      and 2px stroke; `filled` still signals state (bookmark, play)
- [x] Icon URLs in `index.html` and the manifest carry a version, so a
      browser or an installed app that cached the old icon fetches the new
- [x] `logo-dark.svg` with a handle that reads on black; the top bar, Home
      and the SVG favicon (`prefers-color-scheme`) use it in dark mode

## Comments

**2026-11-18 — built.** `icons.ts` holds Lucide's markup for the same 20
names (lucide-static 0.460.0, licence in the file's header), so no icon
package ships and every caller is unchanged. Nav Songs is `library-big`,
the menu is `circle-user-round`, restart `rotate-ccw`. Icon URLs carry
`?v=2` in `index.html` and the manifest — the files were already new, so
a tab or installed app showing the old music note had cached it; Chrome
re-reads an installed app's manifest on its own schedule (about daily),
so an installed copy can take that long. `logo-dark.svg` lightens only the
handle (`#9AA8B2`, ~8.6:1 on black); the top bar and Home pick it by the
app's theme, and `favicon.svg` by the browser's (`prefers-color-scheme`),
since the tab strip follows the browser.
