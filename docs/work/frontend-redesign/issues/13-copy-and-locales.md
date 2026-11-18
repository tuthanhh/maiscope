# 13 — Copy and locales

**What to build:** the strings *Copy and locales* lists: new ones added in all nine locales,
removed ones deleted, nothing left referencing a key that no longer exists.

**Blocked by:** 09, 10, 11, 12.

**Status:** done

- [x] Every new string in all nine locale files — those a screen reads now; the community strings moved to 05
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

**2026-11-18 — review.** Applied: ru `term.noteDesigner` read "designer's
notes" — now "Автор чарта"; maimai's own words for artist and chart
designer in Chinese (曲师/谱师, 曲師/譜師); vi artist "Nghệ sĩ"; play speed
as playback, not gameplay, in es, ru and id; the chart designer in es, ko
and id named with each file's word for a chart; 乐曲/樂曲 in the zh song-list
titles; one verb for loading in zh-Hans (载入) and one word for list in
zh-Hant (清單); sentence case in ru and vi titles. Answered: en keeps
"sheet" and "chart" apart on purpose — a sheet is a difficulty entry in
the Catalog ("No sheets match these filters", as the spec words it), a
chart is its simai text ("No chart yet", "Chart text"). The community
strings move to 05's checklist: no key is shipped that nothing reads.
