# 21 — Settings and account apart; JetBrains Mono back

**What to build:** the maintainer's asks of 2026-11-18: the display face
goes back to JetBrains Mono (from 15's Space Grotesk), and the top bar's
end separates the account from the settings, each behind its own icon.

**Blocked by:** 15, 20.

**Status:** done

- [x] `--f-display` is JetBrains Mono with the CJK fallbacks; Space
      Grotesk is no longer fetched
- [x] A gear (`MvSettingsMenu`) holds Language, About, Report Issue and
      Source Code
- [x] The account (`MvAccountMenu`) stands apart past a hairline: Sign in
      while signed out (phones included), the avatar menu when signed in
- [x] Shared open/close behaviour in `composables/useDisclosure.ts`
- [x] design-direction's "Navigation" and "Type families" updated

## Comments

**2026-11-18 — built.** `MvMenu` became `MvSettingsMenu`; the account
half moved to `MvAccountMenu`, with the delete dialog. The top-bar
"Sign in" button now shows on phones too — it is the account's only
control while signed out — so "Sign in with GitHub" in the menu went, with
`nav.menu`. Body text stays Inter; only the display roles changed. The
sign-in flow passes against the new structure in the session's
Playwright browser checks (scratchpad scripts, not a committed test —
`apps/host` still has none).

**2026-11-18 — review.** Applied: focus goes to the Sign in button after
Sign out, and back to the gear after choosing About, instead of falling to
`<body>`. A stored session that could not be checked (offline, server
down) is `unverified` in the store, and the account slot shows nothing
rather than a "Sign in" that would start a needless OAuth round trip.
