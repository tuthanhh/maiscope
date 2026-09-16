# 02 — Server: endpoint coverage + `/catalog` contract snapshot

**What to build:** Grow the six existing `#[sqlx::test]` handler tests to cover
all eight routes, and add one snapshot test that guards the response contract.

The contract snapshot is the cheap substitute for wiring the `shared` crate.
Generated types would not match what the frontend actually uses anyway — the
server deliberately emits raw fields only, while `preprocessData` adds `songNo`,
`imageUrl`, `sheetExpr`, `notePercents` and `$canonicalSheet` client-side. In a
read-only v1 the only real failure is the server dropping or renaming a field the
client reads, and a snapshot catches exactly that.

**Blocked by:** `server-restructure/issues/04-split-modules.md`, `server-restructure/issues/07-etag-caching.md`

**Status:** in-progress

- [x] Coverage for all eight routes — `/sheets/{sheet}/chart` had **zero** tests
      and now has four; every other route already had some
- [ ] `/sync/delta` `409 snapshot_required` path tested (`since` predating
      `last_full_reload_revision`) — **not done, see below**
- [ ] `/sheets/search` pagination boundary — **not done**. The `X-Total-Count`
      half is covered by `api-pagination-header` 01
- [x] Snapshot test asserting `/catalog`'s top-level shape and one full `Song`
      with nested sheets
- [x] Snapshot asserts the **absence** of derived fields, at any nesting depth
- [x] ETag/304 tests from `server-restructure` issue 07 — already existed (6 on
      `/catalog`, 2 on `/sync/manifest`); folded in by being counted, not rewritten
- [x] Rate-limit key test from `server-restructure` issue 08 — already existed
      (`routes/mod.rs`, plus 7 in `rate_limit.rs`)
- [x] CLAUDE.md corrected (`test-foundation` 01's commit)

## Stopped deliberately

Paused partway. The API is being redesigned — see
[`api-v2`](../api-v2/spec.md) — under transient coexistence: v2 is authored
fresh, the frontend migrates endpoint by endpoint, and v1 is deleted once
nothing calls it.

That does **not** make these tests waste. They are the regression net the
rewrite needs: the `/catalog` snapshot is the before-and-after comparison that
says v2 returns the same data as v1, which is the thing a from-scratch rewrite
most often gets wrong. That is worth more than the two remaining checkboxes.

### The premise was stale

Written against "six existing `#[sqlx::test]` handler tests". There were 121
server tests passing when this was picked up, 16 of them on routes, and four of
the eight checkboxes were already satisfied.

### One checkbox was wrong, not just done

`X-Total-Count` contradicted `api-contract.md` §6, which specified a body field
and gave a reason. Resolved by
[ADR-0015](../../adr/0015-pagination-total-as-a-response-header.md) — the
contract's reason was conditional on being the only paginated endpoint, which
`community-charts` ends — so the header won, but via its own feature rather than
by smuggling an API change into a test ticket.

### What landed

- `/catalog` contract snapshot (`apps/server/tests/snapshots/catalog.json`),
  seeded with one song, two sheets — one fully populated, one sparse — and every
  lookup table, so optional-field handling is covered rather than just the happy
  path. `update_time` is a fixed literal so the snapshot is byte-stable.
- A separate recursive walk asserting the six client-derived fields appear at no
  depth, with a guard proving the walk actually descends into
  `songs[].sheets[]` — otherwise the assertion could pass vacuously.
- Four tests for `/sheets/{sheet}/chart`: verbatim round-trip, `sheet_expr`
  rebuilt from all three parts, `404`, and the `501` blob-only arm.
- `routes/charts.rs` keeps its `(StatusCode, String)` return, now with a comment
  pointing at contract §2. It is the documented plain-text exception, not an
  oversight — converting it would change a shipped endpoint's wire format.

### What is left

`/sync/delta`'s `409` (needs `last_full_reload_revision` seeded directly, since
`sync_catalog` never writes it) and a `/sheets/search` page-boundary test. Both
should be written against v2, not v1.