# ADR-0015 — Pagination total moves to `X-Total-Count`

**Status:** accepted
**Date:** 2026-09-16
**Amends:** the Pagination convention in
[`api-contract.md`](../reference/api-contract.md) §6

## Context

`GET /sheets/search` returns `{ sheets, total }`, and §6 states the convention
with its reason:

> The total is a `total` **field in the response body**, not an `X-Total-Count`
> header — the only paginated endpoint (`GET /sheets/search`) returns an
> envelope already, so a header would be a second place to look.

That reasoning was sound and is now expiring.
[ADR-0013](0013-community-charts-beside-the-catalog.md) adds
`GET /community/songs` and `GET /community/reports`, both paginated. "The only
paginated endpoint" stops being true, and a per-endpoint choice about where the
count lives becomes three chances to be inconsistent.

Two facts constrain how this can change.

**The header is invisible to the browser without CORS help.** `routes/mod.rs`
configures `CorsLayer` with `allow_origin` and `allow_methods` only. A response
header that is not on the CORS safelist cannot be read by JS unless the server
names it in `Access-Control-Expose-Headers`. The frontend is cross-origin —
Cloudflare Pages to Fly — so `response.headers.get("X-Total-Count")` would
return `null` with no error raised anywhere, and a same-origin test would not
catch it.

**The endpoint is shipped and has a live consumer.**
`useSheetSearch.ts:48` reads `data.total`, and `BrowsePagination.vue` computes
its range from it. Removing the field and adding the header in one release
breaks whichever tier deploys second.

## Decision

The paginated total is returned as an `X-Total-Count` response header, on every
paginated endpoint. Pagination stays offset-based (`?page`, `?pageSize`).

The migration is expand-and-contract, the rule `production-v1` already applies to
schema changes:

1. **Expand** — emit `X-Total-Count` *and* keep `total` in the body. Add
   `expose_headers`. Both consumers work; nothing breaks.
2. **Switch** — the frontend reads the header.
3. **Contract** — drop `total` from the body.

Each step is independently deployable, and the order is load-bearing in both
directions.

## Rejected alternatives

**Keep the body field.** Zero work, and it is what the contract says today.
Rejected because the reason it gives is conditional on a fact that ADR-0013
removes.

**Emit both, permanently.** No migration, no breakage, and the header is there
for anyone who wants it. Rejected because it is exactly the "second place to
look" that §6 argued against — with the added hazard that two sources for one
number can disagree after a refactor and nothing would notice.

**Keyset pagination everywhere, dropping `total` entirely.** Correct under
concurrent writes, and it removes the `COUNT(*)` that `/sheets/search` runs on
every query. Rejected for now: it deletes the page-number UI
`BrowsePagination.vue` is built around, and "showing 1–22 of 6559" is worth
keeping for a catalog of known size. See Consequences for when to revisit.

**Per-endpoint choice — body for search, header for community.** Lets each
endpoint do what suits it. Rejected as the worst of both: a convention that is
not a convention, and a client that must remember which is which.

## Consequences

Every future paginated endpoint must set the header *and* be listed in
`expose_headers`. The second half is the one that will be forgotten, because
omitting it fails silently and only cross-origin — which is to say, only in
production.

`SheetSearchResponse` loses a field at step 3, so the `/catalog` contract
snapshot from [`test-foundation` 02](../work/test-foundation/issues/02-server-tests-and-contract-snapshot.md)
gains value: it is an independent check that editing response types did not
disturb a different endpoint's shape.

`community-charts` tickets 03 and 06 were written against the body-field
convention and are updated to this one.

**What would reopen this:** upload volume on `/community/songs` high enough that
a page of results turns over while someone is reading it. Offset pagination
drifts under concurrent writes — a row inserted between a user's page-1 and
page-2 requests shifts every later row down, so page 2 repeats an item and skips
another. At community scale, with uploads minutes or hours apart, the odds are
negligible. If that changes, keyset pagination for that endpoint is worth the
second idiom, and `X-Total-Count` simply stops being sent there.
