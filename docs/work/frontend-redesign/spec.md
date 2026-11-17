# Spec — Frontend redesign

**Status:** active
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Rebuild `apps/host` around what maiscope is now, rather than what it inherited.
Cuts over on the same branch as [`api-rewrite`](../api-rewrite/spec.md).

## Problem

The frontend is a port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs),
originally Nuxt 2 + Vuetify 2, rebuilt on native Vue 3 for maimai only. That
origin still shapes it in ways nothing else justifies:

- **`preprocessData` rehydrates a frozen, prototype-linked object graph** and
  derives `songNo`, `imageUrl`, `sheetExpr`, `notePercents` and
  `$canonicalSheet` client-side. The server sends raw fields only *because* the
  client did this when it fetched a CDN file, not because anyone chose the split.
- **`sheetExpr` is the client's identity key**, so the same title-derived string
  that plagues the API also indexes the UI.
- **`pages/visualizer.vue` is 1047 lines** — the largest file in the app by a
  factor of three, holding the engine bridge, chart loading and manual paste.
- Filtering happens over the whole catalog in memory
  ([ADR-0007](../../adr/0007-client-side-filtering.md)) *and* server-side via
  `/sheets/search`. Both exist; neither is clearly the one to use.

Nothing here is broken for users. It is a v1 shape being carried into a product
that now has uploads, accounts and a second library.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Data requirements per screen](issues/01-data-requirements.md) | **Blocks `api-rewrite`'s payload.** Not code. Written first |
| 06 | [Design direction](issues/06-design-direction.md) | **Bounds the visual work.** Not code. Written first, alongside 01 |
| 02 | [Identity and indexing off `sheetExpr`](issues/02-identity-off-sheetexpr.md) | Needs `song-public-id` |
| 03 | [Split the visualizer page](issues/03-split-visualizer.md) | Independent, doable now |
| 04 | [Screens against the new API](issues/04-screens-on-new-api.md) | Blocked by `api-rewrite` and 06 |
| 05 | [Community library screens](issues/05-community-screens.md) | Blocked by `community-charts` and 06 |
| 07 | [Tokens and type](issues/07-tokens-and-type.md) | The design system's foundation |
| 08 | [Base components](issues/08-base-components.md) | The component inventory |
| 09 | [Navigation and home](issues/09-navigation-and-home.md) | |
| 10 | [Browse](issues/10-browse.md) | Grid only, filter drawer |
| 11 | [Song modal with a URL](issues/11-song-modal.md) | Replaces the song page and sheet dialog |
| 12 | [Chart page](issues/12-chart-page.md) | Around the unchanged playback surface |
| 13 | [Copy and locales](issues/13-copy-and-locales.md) | Last |

Issue 04 turned out to be two jobs: reading the new API, done with issue 02,
and the visual and structural redesign `design-direction.md` describes, too
large for one ticket. 07–13 split the second along the document's own
sections, each checked in a browser against the local API.

Ticket numbers are creation order, not execution order — 01 and 06 are both
documents and both come first. The umbrella's build order is the authority.

## Decisions that constrain this work

- **Ticket 01 comes first and is not code.** "Should the server send raw fields
  or rendered ones?" is unanswerable until the client is specified. Today's
  answer is knowable by reading `preprocessData`; a redesigned client's is not.
  Until that list exists, `api-rewrite` is designing a payload for a consumer
  nobody has described.
- **One branch, one cutover.** The API is replaced wholesale rather than served
  alongside, so the two move together. Neither is deployable alone.
- **Raw-versus-derived is genuinely open.** It was inherited, not chosen. Ticket
  01's output decides it; do not assume the current split survives, and do not
  assume it dies either.
- **The visualizer is not being redesigned.** Ticket 03 is a split for
  legibility, not a rework of playback. The engine boundary
  (`wasm_bridge.rs`'s mailbox, drained once per frame) is unchanged.
- **"Redesign" means visual as well as structural, on the same branch.** Decided
  2026-09-16 against two alternatives: renaming the feature to something
  structural-only, or splitting the visual work into a separate feature shipped
  incrementally after cutover. The deciding argument for one branch is that
  `community-charts` adds upload, browse and reports screens with **no existing
  look to inherit** — building them in the old skin and reskinning later means
  designing them twice.

  The cost is named plainly in the umbrella: this is what makes the branch's
  length unbounded, and branch length is the milestone's largest risk.
- **Ticket 06 is what bounds it.** Taste-driven work ends when it looks right,
  which is not a completion test. The design direction — type scale, colour
  tokens, spacing scale, component inventory — is written first and **committed
  to `docs/reference/`**, so tickets 04 and 05 implement against a checklist
  instead of iterating on taste. A direction that stays in a chat session is not
  a bound.

## Out of scope

The Bevy engine and its rendering. Mobile gating, which
[`mobile-webview-spike`](../mobile-webview-spike/) owns. Adding a test runner —
`apps/host` has no tests and this feature does not change that, though ticket 01
makes it clearer what would be worth testing.

## Done when

Every screen reads the new API, no client code derives an identity key from a
title, and `preprocessData` either has a stated reason to exist or is gone.
