# 05 — Community library screens

**What to build:** browse, detail, upload, and the reports queue for the second
library.

[ADR-0013](../../../adr/0013-community-charts-beside-the-catalog.md) keeps the two
libraries visibly separate: official pages show official charts only, and a fan
chart is never rendered where it could be mistaken for the arcade's.

**Blocked by:** [`community-charts`](../../community-charts/spec.md), 04.

**Status:** todo

- [ ] Community browse and detail, distinct from the official browse
- [ ] Upload: a `maidata.txt` picker, the official-song link chooser, and a
      rejection path that shows *which* difficulty and *which* token failed —
      `ParseError` carries all three, so the UI should not flatten it to
      "invalid chart"
- [ ] Reports: submit from a chart, and a moderator queue
- [ ] Nothing community-shaped appears on an official song or sheet page
- [ ] **Sign-in, failing closed** (contract §4's flow). `/auth/callback` calls
      the API callback only when a `state` is stored in `sessionStorage` *and*
      equals the one GitHub returned; it clears the stored value either way.
      This is the whole login-CSRF defence: the API keeps no OAuth state and
      will mint a session for any valid code, so a check that falls through on
      a missing value signs the victim into the attacker's account. Tested:
      no stored state, a mismatch, and a match
      (`server-auth-github-oauth` 03 review)
- [ ] Anything rendering an uploader shows one fixed placeholder for a null
      `login` — a deleted account — in one component, not per screen
      (carried from `server-auth-github-oauth` 06)
- [ ] Account deletion copy says identity removal, not deletion: published
      charts stay up (`server-auth-github-oauth` 06)
- [ ] The community strings of design-direction "Copy and locales" (Upload,
      Publish, Rejected, Sign in, Reports, Resolve, Official song, optional,
      fan chart of, deleted user, the rejection sentence) in all nine locales —
      moved here from 13, which ships only keys the code reads
