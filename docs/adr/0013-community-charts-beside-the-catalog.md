# ADR-0013 — Community charts live beside the catalog, not in it

**Status:** accepted
**Date:** 2026-09-16

## Context

Phase 2 was always "community charts", but the shape on record
([`api-contract.md`](../reference/api-contract.md) §5, `server-contributions`)
was a *contribution queue*: a user proposes an edit, it sits `pending`, a
moderator approves, it merges into the canonical tables. Six tickets describe it.
None were built.

Two things make that design the wrong one now.

**The intent changed.** The model wanted is anyone uploads, anyone plays — fan
charts as content in their own right, including for songs the official catalog
does not contain. A queue that merges proposals into canonical rows cannot
express a chart that has no canonical row to merge into.

**Moderation does not scale to one person.** This is a solo project. A review
queue makes the maintainer a synchronous dependency on every upload, and
upload-to-visible latency becomes however long until they next open the queue.
[ADR-0002](0002-chart-data-only-no-audio-hosting.md) reached a related
conclusion for audio — "the cost is not storage but moderation" — but its
reasoning was about laundering official audio, which a text chart does not carry.
The throughput problem carries; the legal exposure does not.

The catalog tables are also load-bearing in ways user content would disturb.
`songs.song_id` and `sheets.sheet_expr` are `UNIQUE`, `charts` is
`UNIQUE (sheet_id, format)` — one canonical chart per sheet, so two people cannot
chart the same master. `catalog_sync` owns `catalog_meta.revision`, the
`deleted_songs`/`deleted_sheets` tombstones, and the ETag every client caches
against.

## Decision

Community charts live in their own tables, served by their own endpoints, and
**never enter the catalog's revision stream**.

- `community_songs`, `community_charts`, `community_reports`. `songs`, `sheets`
  and `charts` are untouched, so `catalog_sync` keeps its "upsert by natural key,
  never delete" behaviour unchanged.
- **Published immediately, no review queue.** An upload is visible as soon as
  it is stored; a report queue handles problems after the fact. No `pending`
  state. This is about the absence of a moderation gate, not about pushing
  updates to open pages — the browse list is an ordinary page that reflects
  new uploads on reload.
- **The upload unit is a whole `maidata.txt`** — one song plus its `inote_N`
  difficulties, which is what a contributor already has on disk and what
  `parse_maidata` already reads.
- **Fully separate for display.** `GET /sheets/{expr}` is unchanged; official
  pages show only official charts. `community_songs.official_song_id` records
  which official song a fan chart covers, but only `/community/*` reads it.
- **GitHub OAuth required to upload** (§4), so a takedown has someone to act
  against.
- **Parse on upload; reject on failure**, naming the offending token.

Nothing community-related may touch `/catalog` or `/sync/*`.

## Rejected alternatives

**The review queue as specified in §5.** Nothing bad ever ships, and it is
already written. Rejected because a solo maintainer is the queue, and a site
where uploads appear whenever the owner next logs in is a site people stop
uploading to.

**Trust tiers** — review the first contribution, then let that author publish
directly. Bounded moderation load that shrinks over time, and a genuinely good
answer at larger scale. Rejected as premature: it needs per-user trust state and
a promotion rule to solve a throughput problem that does not exist yet at zero
contributors. Worth reopening if reports outpace the ability to handle them.

**One set of tables with an `origin` column.** One read path, one set of queries.
Rejected because every existing query would need an origin filter or silently
start returning fan content, and `sheet_expr` uniqueness still forbids two people
charting the same sheet — the constraint that made a separate namespace necessary
in the first place.

**Extending `sheetExpr` with a discriminator** so multiple charts can share a
song, type and difficulty. Solves competing charts head-on. Rejected because
`sheetExpr` is the cross-tier key — `utils/sheet.ts:computeSheetExpr`, the
`sheet_expr` column, and every URL — so changing its shape moves all three tiers
for a feature that does not need the catalog's key at all.

**Unified search across both libraries.** Best discovery: one search box finds
everything. Rejected because `queries/sheets.rs` is 766 lines of filter-building
over official columns, and community rows have no category, version or region for
those filters to mean anything against.

**Showing fan charts on official sheet pages.** Content found where the user
already is. Rejected in favour of keeping the two libraries visibly distinct —
an official sheet page should show what the arcade has, and nothing that could be
mistaken for it.

**Automatic title matching to link a fan chart to its official song.** Zero
upload friction, and `seed_songs` already does it. Rejected on this project's own
evidence: the 2026-09-14 production seed run reported `266 unmatched titles, 291
songs skipped entirely` against a catalog that contains those songs
(`prod-data-and-infra` issue 04). With no review step there is no one to catch
a misfile, so the uploader picks the song explicitly.

## Consequences

`server-contributions` is superseded. Five of its six tickets die with the queue;
ticket 04 (presigned audio upload) was already contradicted by ADR-0002 before
this decision and should not be revived by it.

`api-contract.md` §5 needs rewriting from a queue to a library. §4 (GitHub OAuth)
is unaffected and remains a hard prerequisite.

Validation-on-upload makes the parser an admissions policy. Its gaps become
contributor-facing errors — UTAGE `$`/`@` is still unsupported
([`parser-defects` 03](../work/parser-defects/issues/03-utage-tap-modifiers.md)),
so a legitimate UTAGE chart would be refused by our incompleteness rather than
its own. Every rejection is a bug report; that is the intended feedback loop, but
it means parser coverage now has a user-visible cost.

Community charts get no freshness story. The client cache, ETags and
`/sync/delta` cover the catalog only. If the community library ever needs
incremental sync, it needs its own mechanism — reusing `catalog_meta.revision`
would mean every upload invalidates every client's entire catalog cache.

Two libraries mean two browse experiences to build and keep coherent. That cost
is accepted deliberately: it is the price of the separation, not an oversight.
