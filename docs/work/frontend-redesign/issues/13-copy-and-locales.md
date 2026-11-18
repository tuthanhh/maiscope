# 13 — Copy and locales

**What to build:** the strings *Copy and locales* lists: new ones added in all nine locales,
removed ones deleted, nothing left referencing a key that no longer exists.

**Blocked by:** 09, 10, 11, 12.

**Status:** done

- [x] Every new string in all nine locale files
- [x] Every removed string deleted from all nine
- [x] No template references a missing key

## Comments

**2026-11-18 — built.** The nine locale files are regenerated from the keys
the code reads: 74 each, the same set in each, in en.yaml's order. 84 dead
keys are gone (the v1 gallery, timeline, mode selector, combo drawer, sheet
dialog, YouTube search, advanced-search and list/grid strings among them).
47 strings are translated into the eight other locales — the ones this
redesign added, the ones v1 had left English-only (`page.viz.*`, most of
`page.songs.*`), and the English copy that changed (`myListEmpty`,
`filterResultEmpty`, the three filter toggles). `term.sheets` now says
"chart" in es, id, ru and vi, matching the new strings. The check was a
script: every `t("…")` literal in `src/` against every file's flattened
keys; no `t()` call takes a computed key, and nothing uses `<i18n-t>`.

Found on the way: the paste placeholder `(120){4}…` rendered as
`(120)1,2,…` — `{4}` is vue-i18n interpolation. It is written
`{'{'}4{'}'}` now, and the files' header says so.

Not here: the community strings the design lists (Upload, Publish,
Rejected, Sign in, …) land with the screens that read them (05). The
`--mv-*` compat tokens stay: the untouched playback surface and the About
page still read them. The translations are mine, not a native speaker's;
they want a pass by one.
