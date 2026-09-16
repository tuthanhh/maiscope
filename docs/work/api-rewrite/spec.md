# Spec — API rewrite

**Status:** planned
**Milestone:** phase 2

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

- **Mint a stable `public_id` per song.** Never derived from text. `song_id`
  is demoted to upstream's natural key, used only for sync matching.
- **Renames are not chased.** A renamed song is a tombstone plus an insert, as
  today; it is fixed by hand if noticed. A heuristic match on
  `image_name` + `artist` was rejected: 180 non-UTAGE songs share a jacket, so a
  false merge is a live possibility, and a false merge is silent and corrupting
  while a missed rename is visible and recoverable.
- **UTAGE charts are separate songs with one chart each.** Confirmed: 176 songs
  carry a UTAGE sheet, none has more than one. Upstream already prefixes the
  label — `[好]好きな惣菜発表ドラゴン` — in 100 cases and uses `(宴) X` in the
  other 76. That scheme is *not* unique: across 172 local maidata files, 22
  `(label, origin)` pairs collide. Another reason `public_id` is minted rather
  than derived.

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

## Open questions

- **Payload: raw versus derived.** Blocked on the frontend's data requirements —
  "should the server send what the client renders?" has no answer until the
  client is specified. Today's answer is knowable; a redesigned frontend's is not.
- **Does the chart endpoint keep plain-text errors?** Currently justified (the
  engine wants simai text, so a JSON error beside a text success is arguably the
  *less* consistent design), but a from-scratch API should decide rather than
  inherit.
- **Does a UTAGE difficulty appear in URLs as `協` or `【協】`?** The catalog
  stores bracketed; maidata emits bare.

## Blocked by

[`catalog-difficulties`](../catalog-difficulties/spec.md) and
[`song-public-id`](../song-public-id/spec.md) for identity; the frontend's data
requirements for the payload.

## Done when

The frontend calls only `/api/...`, the old routes are deleted, and no URL in
the system contains a percent-encoded title.
