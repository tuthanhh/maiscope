# 19 — Icon set, app icon refresh, dark logo

**What to build:** the maintainer's ask of 2026-11-18, "all of the above":
the in-app UI icons from one maintained set instead of the hand-drawn
paths ported from the prototype; app and browser icons that installed
copies actually pick up; a dark-theme variant of the logo.

**Blocked by:** 15.

**Status:** todo

- [ ] Every `MvIcon` name drawn from Lucide (ISC licence), same 24px grid
      and 2px stroke; `filled` still signals state (bookmark, play)
- [ ] Icon URLs in `index.html` and the manifest carry a version, so a
      browser or an installed app that cached the old icon fetches the new
- [ ] `logo-dark.svg` with a handle that reads on black; the top bar, Home
      and the SVG favicon (`prefers-color-scheme`) use it in dark mode
