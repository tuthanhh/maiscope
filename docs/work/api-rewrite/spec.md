# Spec — API rewrite

**Status:** active
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Replace the HTTP API with one authored from scratch, on a branch, cutting over
together with the redesigned frontend. There is no second API served alongside,
and no version segment in the path: `/api/...`, not `/api/v1/...`.

## Problem

The API grew out of replacing a static `data.json` and its shape still shows
that. The incoherence is small today and about to be doubled — the community
library adds seven endpoints that would inherit whatever conventions exist.

**A sheet has two spellings.** `GET /sheets/{sheetExpr}` takes the composite key
URL-encoded; `GET /sheets/{songId}/chart?type=&difficulty=` takes the song id and
splits the rest into query params. §2 of the contract acknowledges the split.

**The identity key is display text.** `sheetExpr = songId|type|difficulty`, and
`song_id` *is the title* — 1761 of 1845 songs have `song_id = title` exactly.

| | count |
|---|---|
| songs | 1845 |
| `song_id` not URL-safe | 1638 |
| non-ASCII | 1181 |
| containing a space | 655 |
| containing `/` | 5 |

So today's URLs are not `example%7Cdx%7Cmaster` but
`%E5%90%9B%E3%81%AE%E7%9F%A5%E3%82%89%E3%81%AA%E3%81%84%E7%89%A9%E8%AA%9E%7Cdx%7Cmaster`.
Worse than ugly: `catalog_sync` upserts `ON CONFLICT (song_id)`, so an upstream
title correction is an id the table has never seen — insert, not update. The old
row is tombstoned, and its seeded chart text stays attached to the dead
`sheet_expr` while the new row reports `hasChart: false`.

**Conventions are per-endpoint.** Errors are a JSON envelope except where they
are plain text. Pagination's total moved to a header only after
[ADR-0015](../../adr/0015-pagination-total-as-a-response-header.md).

## Decisions

Settled by grilling before any ticket existed. Recorded so they are not
relitigated.

**Shape of the work**

- **Branch, not coexistence.** v2 is developed on a branch and replaces the old
  API at merge; the two never serve simultaneously. Accepted consequence: the
  frontend and server cut over together, so the frontend redesign must be
  finished before anything ships, and the branch is live-or-dead at the end.
  `test-foundation` 02's `/catalog` snapshot is the only mechanical check that
  the new API returns what the old one did — it is worth more under this plan,
  not less.

  **Master stands still while that branch is open** (`production-v2`
  cross-cutting rule 7). The deployable v2 work — `catalog-difficulties`,
  `song-public-id`, `parser-crate-extraction`, `sync-rename-alarm` — merges
  first, and the branch is cut after. A long-lived branch that has to be
  re-synced against a moving master is a coordination cost with no compensating
  throughput when there is one developer, and this branch rewrites the very
  queries that work changes.
- **No version segment.** `/api/...`. With no external consumers and only ever
  one API deployed, a version in the path names a distinction that does not
  exist. §6's "breaking changes → `/api/v2`" rule is retired with it.
- **Module tree in `apps/server`,** sharing `AppError`, state, rate limiting,
  CORS, tracing and `Freshness`. Queries and types reused where shapes genuinely
  match.
  > **Tripwire.** Reuse is how "authored from scratch" dies quietly. If the new
  > `/catalog` response comes out byte-identical to
  > `apps/server/tests/snapshots/catalog.json`, that is a signal to look, not a
  > success.

**Identity**

- **Mint a stable id per song, and make it the primary key.** Never derived
  from text. `song_id` is demoted to upstream's natural key, used only for sync
  matching.
- **Renames are not chased.** A renamed song is a tombstone plus an insert, as
  today; it is fixed by hand if noticed. A heuristic match on
  `image_name` + `artist` was rejected: 180 non-UTAGE songs share a jacket, so a
  false merge is a live possibility, and a false merge is silent and corrupting
  while a missed rename is visible and recoverable.
- **UTAGE charts are separate songs with one chart each.** Confirmed: 176 songs
  carry a UTAGE sheet, none has more than one. Upstream already prefixes the
  label — `[好]好きな惣菜発表ドラゴン` — in 100 cases and uses `(宴) X` in the
  other 76. That scheme is *not* unique: across 172 local maidata files, 22
  `(label, origin)` pairs collide. Another reason the id is minted rather than
  derived.

**Addressing**

```
GET /api/songs/{publicId}
GET /api/songs/{publicId}/sheets/{type}/{difficulty}
GET /api/songs/{publicId}/sheets/{type}/{difficulty}/chart
```

One spelling. `publicId` is ASCII, so the common case carries no encoding at
all; UTAGE difficulty labels are non-ASCII, so those paths encode the last
segment only — 176 songs rather than today's 1638.

**Difficulty**

`Difficulty` becomes `Basic | Advanced | Expert | Master | ReMaster | Utage(String)`
— five closed variants plus an open set, which is what the data is. Validated
against the `difficulties` table rather than a list in code
([`catalog-difficulties`](../catalog-difficulties/spec.md)). Community uploads
must name a difficulty that exists in that table.

## Settled 2026-09-16

- **UTAGE difficulties appear in URLs as `協`, not `【協】`.**
  [`catalog-difficulties`](../catalog-difficulties/spec.md) issue 01 splits the
  column: `name` holds the bracketed display form, `code` holds the bare
  matching form, and `code` is the URL component. Storing one and deriving the
  other would put a strip-or-add transformation on every path — parser output,
  URL parsing, display, upload validation — and one missed application makes a
  chart unfindable. Measured: all 176 UTAGE sheets are exactly `^【.】$`, 50
  distinct labels, some ASCII (`【X】`, `【J】`, `【r】`) and therefore
  case-sensitive.
- **The chart endpoints keep plain-text errors, routed through `AppError`.** The
  original justification stands — the success body is simai for the engine, so a
  JSON error beside it is arguably *less* consistent. What does not stand is the
  bespoke error path it grew: `routes/v1/charts.rs:51` returns a raw `sqlx::Error`
  string to the client, so the documented exception to the error envelope
  quietly became an exception to error redaction too. `AppError` gains a
  plain-text rendering; the wire format is unchanged, the leak closes by
  construction, and a future handler cannot reintroduce it by copying this one.
  Applies to `/api/songs/{publicId}/sheets/{type}/{difficulty}/chart` and to
  `/api/community/charts/{publicId}/chart` alike.
- **Community routes nest under `/api/community/...`.** One root, two named
  libraries — a second library beside the catalog, not a second API. Keeps every
  route reachable under one prefix for CORS, rate limiting and anything later
  mounted per-subtree.
- **The song id is a UUID v7** in a native `uuid` column, canonical hyphenated
  in paths, and it is `songs.id` itself rather than a second column beside a
  bigserial. See [`song-public-id`](../song-public-id/spec.md) issue 01.

## Settled 2026-11-17

The payload question was blocked on the frontend's data requirements; with
[`screen-data-requirements.md`](../../reference/screen-data-requirements.md)
written, the maintainer picked from options. Recorded in
[ADR-0026](../../adr/0026-v2-api-carries-codes-and-read-fields-only.md):

- **Raw, not derived, and only what a screen reads.** The client derives
  `imageUrl`; `songNo`, `imageUrlM`, `notePercents` and `sheetExpr` disappear.
  Fields no screen reads are not sent, `songId` included.
- **A sheet is its song id, type and difficulty code.** No sheet id is minted;
  the data-requirements document's `?sheet=<id>` is corrected to match.
- **Difficulty travels as its `code`**, in payloads as in URLs.
- **`/sync/delta` is dropped**, as `/sheets/search` was by ADR-0021. The
  manifest shrinks to its `revision`.
- **The five reference tables** are `GET /api/reference` and also ride inside
  the Catalog, per ADR-0021.
- **Vanished rows are hidden.** A song or sheet with a standing tombstone is
  not served; the sync clears the tombstone if the row returns. Picked after
  the feature review found delta's removal left no other signal.

## Tickets

| # | Ticket | Note |
|---|---|---|
| 01 | [v1 routes move under `routes/v1/`](issues/01-v1-routes-move-aside.md) | Mechanical; makes room |
| 02 | [Health and reference tables](issues/02-health-and-reference.md) | First `/api` read routes |
| 03 | [The Catalog](issues/03-catalog.md) | The snapshot, reshaped |
| 04 | [One song, one sheet](issues/04-song-and-sheet.md) | The addressed reads |
| 05 | [Chart text](issues/05-chart-text.md) | Plain-text errors through `AppError` |
| 06 | [Sync manifest](issues/06-sync-manifest.md) | Revision probe only |
| 07 | [Delete `/api/v1`](issues/07-delete-v1.md) | Last; blocked on the frontend cutover |

Each ticket updates `api-contract.md` for the routes it adds. Until 07, the
contract describes both trees.

## Blocked by

[`catalog-difficulties`](../catalog-difficulties/spec.md) and
[`song-public-id`](../song-public-id/spec.md) for identity; the frontend's data
requirements for the payload.

## Done when

The frontend calls only `/api/...`, the old routes are deleted, and no URL in
the system contains a percent-encoded title.
