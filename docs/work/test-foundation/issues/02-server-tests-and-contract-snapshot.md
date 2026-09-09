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

**Status:** todo

- [ ] Coverage for all eight routes, including `/sync/manifest` and `/sync/delta`
- [ ] `/sync/delta` `409 snapshot_required` path tested (`since` predating
      `last_full_reload_revision`)
- [ ] `/sheets/search` pagination + `X-Total-Count` tested
- [ ] Snapshot test asserting `/catalog`'s top-level shape and one full `Song`
      with nested sheets
- [ ] Snapshot asserts the **absence** of derived fields — the server must never
      start emitting `sheetExpr`/`imageUrl`/`notePercents`
- [ ] ETag/304 tests from `server-restructure` issue 07 folded in here
- [ ] Rate-limit key test from `server-restructure` issue 08 folded in here
- [ ] CLAUDE.md corrected — it claims no tests exist beyond `shared`'s scaffold
