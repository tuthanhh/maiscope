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
| Song identity | `song_id` (the title) as natural key, bigserial `id` as PK | `songs.id` **is** a minted UUID v7 — one id per row, hyphenated in URLs |
| Sheet address | `sheetExpr` URL-encoded, *or* songId + query params | `/api/songs/{publicId}/sheets/{type}/{difficulty}` |
| Difficulty | 5-variant enum over a `TEXT` column holding 55 values | `Basic..ReMaster \| Utage(String)`, checked against `difficulties` |
| Difficulty spelling | `【協】` on sheets, `協` from maidata, no reconciliation | `difficulties.name` (`【協】`, display) and `difficulties.code` (`協`, matching + URLs) |
| Charts | official only, one per sheet | plus a community library in its own tables, under `/api/community/...` |
| Parser | inside the Bevy crate | `crates/simai`, usable by the server |
| Writes | none | authenticated uploads, published immediately |
| Accounts | none | GitHub OAuth; deletion is a tombstone, not a row removal |

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

   The mint is **UUID v7 in a native `uuid` column, canonical hyphenated in
   URLs**, and it **is the primary key** — `songs.id`, `users.id`,
   `user_sessions.id`, `community_songs.id`,
   `community_charts.id`. One scheme across both libraries, and one id per row
   rather than a bigserial with a `public_id` beside it. Two ids for one row is
   two things to keep in step, two things to pick the wrong one of in a query,
   and a standing question of which one any given function takes.

   v7 over v4 because the embedded timestamp publishes only the creation time,
   which both libraries already display; a native `uuid` over `TEXT` because a
   distinct Rust type makes "passed a `song_id` where a song id belongs" a
   compile error rather than a silent lookup miss. A short bespoke id was
   rejected: 3× shorter URLs are not worth a generator, a format `CHECK` and
   collision handling that this project would own and test itself. sqlx already
   builds with its `uuid` feature; only the `uuid` crate is new.

   **The primary key is therefore identity-bearing, and that is a new hazard.**
   Previously a re-insert re-minted a bigserial nothing linked to. Now it
   changes every URL for that row, so any path that re-inserts a song — notably
   an identity-breaking full reload, which `catalog_meta.last_full_reload_revision`
   exists to signal — must reuse the existing id by matching on `song_id`. That
   has to be a tested property, not a convention.

   Bigserial keys survive where nothing addresses the row publicly: `sheets`,
   `charts`, the `sheet_*` tables, `users` and `community_reports`. Only
   `sheets.song_id_fk` widens, the single column in the schema referencing
   `songs(id)`.
5. **v1's additive-only response rule does not apply.** It existed because an
   installed PWA could run an arbitrarily old frontend against a current API.
   v2 replaces both on one branch, so the constraint is lifted — deliberately,
   not by omission. It returns the moment anything but this project's own
   frontend consumes the API.
6. **Deleting an account strips identity; it does not delete published work.**
   A deletion nulls `login` and `avatar_url`, sets `deleted_at` and deletes
   every `user_sessions` row, keeping the row so uploads stay attributed and a
   moderator still has a target. `github_id` is *retained* — without it a
   re-login mints a second account and the tombstone is unreachable — so
   re-authenticating with the same GitHub account reactivates the tombstone.
   This is identity erasure, not erasure; say so plainly wherever it is
   described to a user.
7. **One branch, and master stands still while it is open.** See *The cutover
   is the risk*. This is a rule rather than a preference because every
   alternative spends a solo developer's time on merges that a team structure
   would be paying for.

## The cutover is the risk

v2's API and frontend are replaced together, on one branch, with no
intermediate shippable state. This reverses the transient-coexistence plan
considered first, and it is the single largest risk in the milestone.

What makes it survivable:

- [`test-foundation`](../test-foundation/spec.md)'s `/catalog` snapshot is the
  mechanical before-and-after check that the new API returns what the old one
  did. It is worth more under this plan than under a gradual one.
- `catalog-difficulties`, `song-public-id`, `parser-crate-extraction` and
  [`sync-rename-alarm`](../sync-rename-alarm/spec.md) are all deployable against
  the *current* app. Only `api-rewrite`, `frontend-redesign` and
  `community-charts` are branch-bound.
- The engine is untouched by the cutover.
- **The branch is not cut until the master-side work is merged.** Every
  deployable item above lands on master first; only then is the v2 branch
  created, and master does not move again until it merges back. A long-lived
  branch that has to be re-synced with a moving master is a coordination cost
  with no compensating throughput when there is one developer. The branch cannot
  fall behind a master that is standing still.

What does not:

- The branch is live-or-dead at the end, and its length is bounded by the
  frontend redesign — which now carries a **visual** redesign as well as a
  structural one, and so is bounded in turn by the design-direction reference
  (`frontend-redesign` issue 06). Without that document committed, the branch's
  completion test is subjective, which is the definition of a branch that does
  not land.
- **Production gets no fixes while the branch is open.** Accepted knowingly: v1
  is browse-only and stable, so the exposure is small. If something does break,
  the choice is a hotfix on master — paying the merge cost once — or living with
  it until cutover. Decide that in the moment; do not discover it mid-outage.

## Verify before relying on (facts I do not trust)

- **How often upstream actually renames a song.** The whole rename-handling
  decision rests on it being rare, and the project has no record either way. If
  it turns out common, the "fix it by hand" answer fails quietly — a renamed
  song loses its chart and reappears as new, and because the sync never deletes,
  the catalog shows the song *twice*: once under the dead title holding the
  chart text, once under the live title reporting `hasChart: false`.

  This is now being measured rather than assumed.
  [`sync-rename-alarm`](../sync-rename-alarm/spec.md) fails the sync run when
  `songs_vanished > 0` and `songs_inserted > 0` coincide, which is the rename
  signature, and both counters already exist in `SyncStats`. It is deployable
  now, so by the time `song-public-id` lands the number will be known.
- Whether `catalog_sync`'s wholesale `DELETE`/re-insert of lookup tables can
  keep synthesised UTAGE difficulty rows without a special case.
- Whether any upstream `songId` will ever exceed what a path segment tolerates
  *after* `public_id` lands — it should not matter, but `song_id` remains the
  sync's conflict key.
- GitHub OAuth's actual free-tier and rate-limit behaviour for a project with no
  organisation behind it.

## Known-unresolved risks

- **The parser is now an admissions policy, and its floor is measured.** Across
  the 1928 local `maidata.txt` files, **56 use the tap-star `$`** that
  [`parser-defects` 03](../parser-defects/issues/03-utage-tap-modifiers.md)
  does not accept — and **19 of those are ordinary non-UTAGE charts**. So that
  ticket is not a UTAGE nicety; it is a rejection rate on normal content, and
  that ticket's own note calling it "lowest value of the five" was written
  before the parser gated uploads.

  2.9% is a *floor*, not the rate: it counts one known defect, and nobody has
  run `parse_chart` over the whole corpus — `crates/simai/tests/corpus.rs` sweeps nine
  hand-written fixtures plus a gitignored `local/` that takes chart bodies, not
  `maidata.txt`. Gating `community-charts` 02 on a measured pass rate was
  considered and **rejected**: the endpoint ships as designed and defects are
  fixed as they are reported. The rejection is not graceful — one `$` anywhere
  refuses every difficulty in that upload — so the contributor-facing error must
  carry the token, and every rejection logs at `warn` with it (see
  `community-charts` issue 02).
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
  of needing a privacy policy. Accounts end that. What is stored is settled —
  `github_id`, `login`, `avatar_url`, `role`, a refresh-token *hash*, and no
  email — and deletion is settled by cross-cutting rule 6. What is not settled
  is whether that set obliges a privacy policy anywhere it has users.

## Before the first v2 deploy

The branch merges and deploys as one. These are not code, so no test catches
them; the machine refuses to start without the first three, which surfaces as
a crash loop while the previous release keeps serving.

- [ ] Create the production GitHub OAuth app with the callback
      `https://maiscope.pages.dev/auth/callback`; put its client id in
      `fly.toml` (`GITHUB_CLIENT_ID`, commented out until then)
- [ ] Set `GITHUB_CLIENT_SECRET` and `JWT_SECRET` as Fly secrets in the
      dashboard (`openssl rand -base64 48` for the latter)
- [ ] The frontend's `/auth/callback` route exists and fails closed on
      `state` (`frontend-redesign` 05)
- [ ] `backup-database.yml` before `deploy.yml`, per ADR-0024. Its runbook
      covers the song-id migration; the deploy also runs the difficulty,
      `users`, revision-bump and sheet-address migrations, all additive. The
      sheet-address `UNIQUE` aborts the release command if production ever
      held two sheets at one (song, type, difficulty) — impossible while
      `sheet_expr` is unique, but that is the failure to recognise
- [ ] After the deploy: `/api/reference`, `/api/catalog` and
      `/api/sync/manifest` answer, and the manifest's revision moved by one

## Accepted, not solved

Each of these was put as a decision and deferred on purpose. They are listed so
a later reader does not mistake a choice for an oversight.

- **Inherited from v1.0** (shipped 2026-11-17): twelve chart folders name
  songs the catalog does not hold, so `seed-charts.yml` still runs with
  `allow_unmatched` and 12 is its known floor — `community-charts` decides
  what a chart without a catalog song is. And two server tests
  (`test-foundation` 02: the `/sync/delta` 409 path, the search pagination
  boundary) were deferred to the API rewrite that replaces both endpoints.

- **No ban or suspension in v2.** `users.role` stays `user | moderator | admin`.
  The only tools against a repeat abuser are per-song takedown and the reports
  queue. Deferred to a later release because `ALTER TABLE users ADD COLUMN
  banned_at TIMESTAMPTZ` is nullable, instant and needs no table rewrite even
  against live accounts — nothing is lost by waiting.

  **When it lands, reactivation must clear `deleted_at` and not `banned_at`.**
  Otherwise cross-cutting rule 6 hands every banned user an escape: delete the
  account, re-authenticate, arrive clean.
- **The JWT is authoritative for 15 minutes.** `AuthUser` verifies the token and
  reads no database, so a role change — or a deletion — takes up to one token
  lifetime to bite. A per-write `SELECT` on `users` was the alternative and was
  not taken.
- **No per-user write quota.** Uploads are covered by the one global limiter
  (burst 30, refill 1/s, keyed by IP), which permits roughly 86,400 uploads per
  IP per day — four orders of magnitude above real use, and keyed by address
  when auth means the user id is known. The bounded piece is the body: 512 KB on
  the upload route, chosen from the corpus (p99 38 KB, observed max 353 KB), not
  from Axum's 2 MB default.
- **Refresh tokens live in client storage.** Not a preference — an httpOnly
  cookie is unavailable on the current hosts. `pages.dev` and `fly.dev` are both
  Public Suffix List entries, so a cookie set by `maiscope-api.fly.dev` while
  the user is on `maiscope.pages.dev` is third-party; Safari blocks it outright.
  A custom domain would make same-site cookies possible and stays **optional** —
  `CORS_ALLOWED_ORIGINS` is already config-driven, so adopting one needs no code
  change. Until then XSS on the frontend origin is credential theft, bounded by
  a 30-day `user_sessions.expires_at` and rotation on every use.

## Child features

This directory is an **umbrella**: cross-cutting rules above, no tickets of its
own. Build roughly in this order — the ordering is the dependency graph, not a
preference.

The order is split by *where the work lands*, because master freezes when the
branch is cut (cross-cutting rule 7).

**On master, before the branch exists.** Each ships on its own against the
current app, and each removes a dependency from the branch-bound work.

| Order | Feature | Why here |
|---|---|---|
| 1 | [`frontend-redesign`](../frontend-redesign/spec.md) tickets 01 and 06 | Two documents, no code. Ticket 01 is the data-requirements list and blocks `api-rewrite`'s payload; ticket 06 is the design direction and bounds the visual work. Neither depends on anything, so both can be written alongside items 2–5. |
| 2 | [`catalog-difficulties`](../catalog-difficulties/spec.md) | Fixes a live bug: `/catalog` ships 5 difficulties while its sheets use 55, and `seed_songs` silently skips every UTAGE chart. |
| 3 | [`song-public-id`](../song-public-id/spec.md) | Mint and backfill. Mechanical, independent, and everything in `api-rewrite` depends on it. |
| 4 | [`parser-crate-extraction`](../parser-crate-extraction/spec.md) | Gets the parser off Bevy so the server can validate uploads. Earns its place on CI cost alone. |
| 5 | [`sync-rename-alarm`](../sync-rename-alarm/spec.md) | One ticket. Starts measuring rename frequency now, so the number exists before the decision that rests on it is load-bearing. |

**On the v2 branch, cut once the above is merged.**

| Order | Feature | Why here |
|---|---|---|
| 6 | [`server-auth-github-oauth`](../server-auth-github-oauth/spec.md) | Hard prerequisite for any write endpoint. |
| 7 | [`api-rewrite`](../api-rewrite/spec.md) | Needs 1, 2 and 3. |
| 8 | [`frontend-redesign`](../frontend-redesign/spec.md) tickets 02–14 | Cuts over with 7, against the direction written in ticket 06. |
| 9 | [`community-charts`](../community-charts/spec.md) | Needs 4, 6, and 7's conventions. |

Auth moved onto the branch rather than ahead of it: nothing on master consumes
it, so shipping login to production months before a single write endpoint exists
would put an unused attack surface in front of users for no return.

[`server-contributions`](../server-contributions/spec.md) is **superseded** and
holds no work — kept as the record of the moderation-queue design that ADR-0013
replaced.
