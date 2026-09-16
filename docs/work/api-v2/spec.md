# Spec — API v2

**Status:** planned
**Milestone:** phase 2

Author a new HTTP API from scratch at `/api/v2`, migrate the frontend to it
endpoint by endpoint, and delete `/api/v1` once nothing calls it.

## Problem

v1 grew from replacing a static `data.json`, and it shows. The incoherence is
small today and about to be doubled: [`community-charts`](../community-charts/spec.md)
adds seven endpoints that would inherit whatever conventions v1 has.

Three concrete warts:

**A sheet has two spellings.** `GET /sheets/{sheetExpr}` takes the composite key
URL-encoded (`|` → `%7C`); `GET /sheets/{songId}/chart?type=&difficulty=` takes
the song id and splits the rest into query params. Same resource, two ways to
name it — §2 acknowledges this outright.

**Conventions are per-endpoint, not global.** Errors are a JSON envelope except
where they are plain text (§2, §6). Pagination put its total in the body for a
reason that expired
([ADR-0015](../../adr/0015-pagination-total-as-a-response-header.md)).

**The payload contract was inherited, not chosen.** The server sends raw fields
only because `preprocessData` came from arcade-songs. The frontend is also being
redesigned, so that constraint is expiring.

## Decisions already taken

Settled by grilling before this spec existed. Recorded so they are not relitigated.

- **Transient coexistence, not a big-bang rewrite.** v2 serves alongside v1
  *during migration only*; the frontend moves endpoint by endpoint; a v1 route
  is deleted once nothing calls it. v1 is still removed — at the end, as a
  deletion, rather than at the start, as a cliff. The alternative, building both
  tiers on a branch and cutting over at once, has no intermediate shippable
  state; this repo already rejected that shape for the server
  ([ADR-0005](../../adr/0005-restructure-server-in-place.md)).
- **v1 is frozen.** No further changes land in it, which is also what stops §6's
  versioning rule ("breaking changes → `/api/v2`") from being violated.
  `api-pagination-header` ticket 03 is dropped for this reason.
- **`/catalog` keeps its role, not its bytes.** One full snapshot, ETag /
  `If-None-Match` / `304`, `max-age=3600`, feeding an IndexedDB cache with
  client-side filtering ([ADR-0007](../../adr/0007-client-side-filtering.md)).
  The body changes with every other payload. Keeping the bytes would mean a v2
  where one endpoint speaks v1's payload language and the rest speak a new one —
  two conventions inside one version, which is worse than the inconsistency v2
  exists to fix.
- **Delivery model is out of scope.** The snapshot-plus-revision-probe design is
  working and is not what is broken.
- **The frontend's data requirements come first.** "Should the server send raw
  fields or rendered ones?" cannot be answered without knowing what the client
  renders. Today's answer is knowable; a redesigned frontend's is not. A short
  list of fields-per-screen is the input that makes this evidence rather than
  invention.

## Open questions

To be ground out before any ticket is written. Each has a recommendation
attached from the first pass.

1. **Addressing.** Hierarchical (`/songs/{songId}/sheets/{type}/{difficulty}`,
   chart as a sub-resource) versus uniform `sheetExpr` everywhere versus opaque
   ids. *Leaning hierarchical* — it removes `%7C` from URLs, gives one spelling,
   and extends to `/community/...`. It does not delete `sheetExpr`:
   `sheets.sheet_expr` stays the `UNIQUE` storage key and
   `utils/sheet.ts:computeSheetExpr` stays the client's identity key. Only its
   use as a URL component goes. *Opaque ids are the trap* — row identity is
   exactly what this system does not guarantee across an identity-breaking
   reload, which is what `last_full_reload_revision` exists for.
   **Unverified fact blocking this:** whether any real `song_id` contains
   characters needing percent-encoding. If so, hierarchical trades one encoded
   segment for another. Needs checking against catalog data.
2. **What "consistency" commits v2 to.** Errors, pagination, envelopes — and
   specifically whether the chart endpoint's plain-text exception survives. It
   is currently justified (the engine wants simai text, so a JSON error beside a
   text success is arguably the *less* consistent design), but a from-scratch
   API should decide that rather than inherit it.
3. **Payload — raw versus derived.** Blocked on the frontend's data
   requirements, per the decision above.
4. **Where community charts sit.** ADR-0013's endpoints were designed against
   v1's conventions. They should be authored as v2 natively rather than
   migrated, which likely means `community-charts` waits for this.

## Out of scope

The frontend redesign itself. This feature needs its *data requirements* — a
list of what each screen renders — not its implementation.

## Done when

`/api/v2` serves every route the frontend uses, the frontend calls nothing under
`/api/v1`, and `/api/v1` is deleted.
