# Spec — Production v2.0

**Status:** planned
**Milestone:** v2.0

Decision record from the 2026-09-16 grilling session. Every choice below was
made explicitly; the rationale is kept because the *reasons* constrain future
work more than the choices do.

## Product shape

Milestones are in [`docs/ROADMAP.md`](../../ROADMAP.md). This is the **umbrella
spec for v2.0**: it holds the rules and risks spanning every child feature listed
at the bottom, and owns no tickets itself.

v1.0 is a browse-only mirror of the official maimai catalog. v2.0 makes maiscope
a place people put charts *into*: accounts, a second library of community charts,
an API authored from scratch, and a frontend rebuilt around all of it.

**Explicitly out of v2**: contributor audio
([ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md), reaffirmed by
[ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md)), a moderation
queue, and any change to the catalog's delivery model — the full-snapshot plus
revision-probe design is working and is not what is broken.

## Architecture

| Layer | v1.0 | v2.0 |
|---|---|---|
| API path | `/api/v1/...` | `/api/...` — the version segment is retired |
| Song identity | `song_id`, which *is the title* | minted `public_id`, ASCII, never derived from text |
| Sheet address | `sheetExpr` URL-encoded, *or* songId + query params | `/api/songs/{publicId}/sheets/{type}/{difficulty}` |
| Difficulty | 5-variant enum over a `TEXT` column holding 55 values | `Basic..ReMaster \| Utage(String)`, checked against `difficulties` |
| Charts | official only, one per sheet | plus a community library in its own tables |
| Parser | inside the Bevy crate | `crates/simai`, usable by the server |
| Writes | none | authenticated uploads, published immediately |

### Decisions behind this scope

| Decision | ADR |
|---|---|
| One marker of each kind per simai token | [ADR-0012](../../adr/0012-one-marker-of-each-kind-per-simai-token.md) |
| Community charts live beside the catalog, not in it | [ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md) |
| The simai parser is its own crate | [ADR-0014](../../adr/0014-simai-parser-as-its-own-crate.md) |
| Pagination total as a response header | [ADR-0015](../../adr/0015-pagination-total-as-a-response-header.md) |

## Cross-cutting rules

1. **Contradictory input is rejected, not guessed at.** Established by ADR-0012
   for duplicate meta markers, extended to partially bracketed slide chains and
   to uploads whose chart text will not parse. The parser is the admissions
   policy; its gaps become contributor-facing errors, which is the intended
   feedback loop and also a cost.
2. **Community content never enters the catalog's revision stream.** `/catalog`
   and `/sync/*` mirror upstream, and their ETag and revision machinery is owned
   end-to-end by `catalog_sync`. A user upload bumping `catalog_meta.revision`
   would invalidate every client's whole catalog cache and inject rows the sync
   cannot reconcile.
3. **`songs`, `sheets` and `charts` are not modified by community work.** Not a
   column, not an index. The separation is what keeps `catalog_sync` unchanged.
4. **Identity is minted, never derived.** Anything computed from a title
   inherits the instability that motivated v2 in the first place. This applies
   to song ids, and to any identifier community charts acquire.
5. **v1's additive-only response rule does not apply.** It existed because an
   installed PWA could run an arbitrarily old frontend against a current API.
   v2 replaces both on one branch, so the constraint is lifted — deliberately,
   not by omission. It returns the moment anything but this project's own
   frontend consumes the API.

## The cutover is the risk

v2's API and frontend are replaced together, on one branch, with no
intermediate shippable state. This reverses the transient-coexistence plan
considered first, and it is the single largest risk in the milestone.

What makes it survivable:

- [`test-foundation`](../test-foundation/spec.md)'s `/catalog` snapshot is the
  mechanical before-and-after check that the new API returns what the old one
  did. It is worth more under this plan than under a gradual one.
- `catalog-difficulties`, `song-public-id` and `parser-crate-extraction` are all
  deployable against the *current* app. Only `api-rewrite`, `frontend-redesign`
  and `community-charts` are branch-bound.
- The engine is untouched by the cutover.

What does not:

- The branch is live-or-dead at the end, and its length is bounded by the
  frontend redesign, which is the least specified piece.

## Verify before relying on (facts I do not trust)

- **How often upstream actually renames a song.** The whole
  rename-handling decision rests on it being rare, and the project has no
  record either way. If it turns out common, the "fix it by hand" answer fails
  quietly — a renamed song loses its chart and reappears as new.
- Whether `catalog_sync`'s wholesale `DELETE`/re-insert of lookup tables can
  keep synthesised UTAGE difficulty rows without a special case.
- Whether any upstream `songId` will ever exceed what a path segment tolerates
  *after* `public_id` lands — it should not matter, but `song_id` remains the
  sync's conflict key.
- GitHub OAuth's actual free-tier and rate-limit behaviour for a project with no
  organisation behind it.

## Known-unresolved risks

- **The parser is now an admissions policy.** UTAGE `$`/`@` is still unsupported
  ([`parser-defects` 03](../parser-defects/issues/03-utage-tap-modifiers.md)), so
  a legitimate UTAGE chart is refused by our incompleteness rather than its own.
- **UTAGE title matching may not be solvable.** Three forms disagree —
  `[宴/バディ/1P] X [13?]`, `(宴) X`, `[好]X` — and matching is already too
  strict. Loosening it trades a missed chart for a *mismatched* one, which is
  worse: the wrong chart rendered as if correct.
- **`(label, origin)` is not unique.** Across 172 local maidata files, 22 pairs
  collide. Community uploads cannot be handed that scheme as an identifier.
- **Moderation is reactive.** Publishing on upload with a report queue means
  problems are found after they are visible. Trust tiers were considered and
  deferred; revisit if reports outpace the ability to handle them.
- **No PII rule is gone.** v1 held none by design, which kept the project clear
  of needing a privacy policy. Accounts end that. Decide what is stored, and
  what is not, before the first user exists — not after.

## Child features

This directory is an **umbrella**: cross-cutting rules above, no tickets of its
own. Build roughly in this order — the ordering is the dependency graph, not a
preference.

| Order | Feature | Why here |
|---|---|---|
| 1 | [`frontend-redesign`](../frontend-redesign/spec.md) ticket 01 | The data-requirements list. Not code, and it blocks `api-rewrite`'s payload — start it first even though the rest of the feature comes last. |
| 2 | [`catalog-difficulties`](../catalog-difficulties/spec.md) | Fixes a live bug: `/catalog` ships 5 difficulties while its sheets use 55, and `seed_songs` silently skips every UTAGE chart. Deployable today. |
| 3 | [`song-public-id`](../song-public-id/spec.md) | Mint and backfill. Mechanical, independent, and everything in `api-rewrite` depends on it. Deployable today. |
| 4 | [`parser-crate-extraction`](../parser-crate-extraction/spec.md) | Gets the parser off Bevy so the server can validate uploads. Earns its place on CI cost alone. Deployable today. |
| 5 | [`server-auth-github-oauth`](../server-auth-github-oauth/) | Hard prerequisite for any write endpoint. |
| 6 | [`api-rewrite`](../api-rewrite/spec.md) | Needs 1, 2 and 3. Branch-bound from here on. |
| 7 | [`frontend-redesign`](../frontend-redesign/spec.md) tickets 02–05 | Cuts over with 6. |
| 8 | [`community-charts`](../community-charts/spec.md) | Needs 4, 5, and 6's conventions. |

Items 2, 3 and 4 are the ones worth starting now: each ships on its own against
the current app, and each removes a dependency from the branch-bound work.

[`server-contributions`](../server-contributions/spec.md) is **superseded** and
holds no work — kept as the record of the moderation-queue design that ADR-0013
replaced.
