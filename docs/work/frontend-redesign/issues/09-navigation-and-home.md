# 09 — Navigation and home

**What to build:** *Navigation* and *Home*: three
destinations (Songs, Community, Play), a top bar on desktop and a bottom bar
on phones, theme toggle, the avatar slot (Sign in until auth lands in the
UI), Locale and About moved into the menu; Home with the brand in `display`.

**Blocked by:** 08.

**Status:** done

- [x] Top bar `--nav-h`, hairline below; active destination `--fg`, others `--fg-muted`
- [x] Phone: slim top bar with brand and avatar, bottom bar with icon-above-label, hidden while the engine canvas is in view
- [x] Community points at a placeholder until `community-charts` exists
- [x] Home: brand, subtitle, primary Songs and ghost Play

## Comments

**2026-11-18 — built.** `components/layout/MvNavigation.vue` (top bar; on
phones a slim top bar plus the bottom bar) and `MvMenu.vue` (Language as a
select, About, and the two external links the old header carried). The
footer is gone: the design has one bar of chrome. The bottom bar hides via
an IntersectionObserver on `#bevy` while the Play route is open
(`useEngineInView`). Sign in and Reports are not in the menu yet: accounts
reach the UI in issue 05, so the menu button shows a generic user icon
rather than an avatar. `/community` is a placeholder page with one sentence.
New strings exist in English only; issue 13 adds the other locales (the
fallback is English). `MvLocaleMenu` is deleted, replaced by the menu's
select. Checked at 1280 and 390 px wide: the bar, the menu, the bottom bar
showing on Community and hidden on Play.

**2026-11-18 — review.** Applied: the menu's language select read a copy of
the i18n facade taken at setup and went stale after a switch — it now reads
the facade live, and the switch fires `LocaleChanged` again. The menu is a
disclosure (`aria-controls`, no `role="menu"`); it closes on a `pointerdown`
outside and on Escape heard at the document, which returns focus to the
trigger. About closes it (fixed by 08's click forwarding). The phone bottom
bar is hidden outright with `visibility: hidden` and `inert` rather than slid
away, and the main area drops its bottom padding while it is hidden. Songs
stays active on the old `/song` page until 11 replaces it. `useEngineInView`
ignores a frame wait that resolves after a route change. `viewport-fit=cover`
makes the safe-area inset real; Home fills one screen on phones without
scrolling.
