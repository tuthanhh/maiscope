# Spec — Community charts

**Status:** planned
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

A second library, beside the official catalog: anyone uploads a `maidata.txt`,
anyone plays it. Decided in
[ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md), which
supersedes the contribution-queue design in
[`server-contributions`](../server-contributions/spec.md).

## Problem

Phase 2's plan of record was a moderation queue: propose an edit, wait for
approval, merge into canonical. It cannot express a fan chart for a song the
catalog does not contain — there is no canonical row to merge into — and it makes
a solo maintainer a synchronous dependency on every upload.

The catalog's own constraints also forbid the content. `charts` is
`UNIQUE (sheet_id, format)`: one chart per sheet, so two people cannot chart the
same master. `sheets.sheet_expr` and `songs.song_id` are `UNIQUE` and owned by
`catalog_sync`.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Tables migration](issues/01-community-tables-migration.md) | Blocked by auth's users table |
| 02 | [`POST /community/songs`](issues/02-upload-maidata.md) | Parse, validate, reject with token |
| 03 | [Browse and detail](issues/03-browse-and-detail.md) | |
| 04 | [Chart text endpoint](issues/04-chart-text-endpoint.md) | `text/plain`, per contract §2 |
| 05 | [Takedown](issues/05-takedown.md) | Uploader or moderator |
| 06 | [Reports queue](issues/06-reports-queue.md) | Replaces the approval queue |
| 07 | [Doc sync](issues/07-doc-sync.md) | Contract §5 rewrite, schema.md, ROADMAP |

## Decisions that constrain this work

All from ADR-0013 unless noted.

- **Nothing here touches `/catalog` or `/sync/*`.** Those mirror upstream and
  their revision/ETag machinery is owned end-to-end by `catalog_sync`. A user
  upload must never bump `catalog_meta.revision` — it would invalidate every
  client's whole catalog cache and inject rows into the delta stream that
  `catalog_sync` cannot reconcile.
- **`songs`, `sheets`, `charts` are not modified.** Not a new column, not a new
  index. The separation is what keeps `catalog_sync` unchanged.
- **Official pages show only official charts.** `GET /sheets/{expr}` is
  unchanged. `community_songs.official_song_id` exists but only `/community/*`
  reads it.
- **Published immediately, no review queue.** No `pending` state anywhere in
  the schema. This concerns the absence of a moderation gate; the browse list
  is an ordinary paginated page, not a self-updating feed.
- **Parse on upload, reject on failure.** The server accepts exactly what the
  engine can render — one parser, shared via
  [`parser-crate-extraction`](../parser-crate-extraction/spec.md).
- **Auth is a hard prerequisite.** Every write needs a GitHub identity so a
  takedown has someone to act against.

## Out of scope

Contributor audio — [ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md),
reaffirmed by ADR-0013. `server-contributions` ticket 04 specified a presigned
upload endpoint; it is dead and must not be revived by this feature.

Incremental sync for the community library. The catalog's manifest/delta tier
covers the catalog only; if community browsing ever needs freshness it gets its
own mechanism.

Unified search across both libraries, and any display of fan charts on official
pages. Both rejected in ADR-0013.

## Done when

An authenticated user uploads a `maidata.txt`, it appears in the community
browse immediately, its charts load in the visualizer, and a malformed upload is
refused with the token that broke it. `cargo tree -p server | grep bevy` is still
empty, and `GET /catalog`'s contract snapshot
([`test-foundation` 02](../test-foundation/issues/02-server-tests-and-contract-snapshot.md))
still passes unchanged.
