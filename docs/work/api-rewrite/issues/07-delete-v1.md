# 07 — Delete `/api/v1`

**What to build:** the cutover's server half. Once the frontend calls only
`/api/...`, the v1 tree goes, with everything only it used.

**Blocked by:** 02, 03, 04, 05, 06;
`frontend-redesign/issues/04-screens-on-new-api.md` (the client must have
stopped calling v1).

**Status:** todo

- [ ] `rm -r apps/server/src/routes/v1/`; `routes/mod.rs` nests `/api` only
- [ ] The router tests in `routes/mod.rs` that drive `/api/v1` paths are
      repointed at v2 routes, not deleted — notably
      `healthcheck_is_exempt_while_the_same_ip_is_throttled_elsewhere`, the
      only proof the health exemption is wired, which burns its bucket on
      `/api/v1/sync/manifest`
- [ ] Queries, types and domain code used only by v1 go with it: sheet search
      and its params, `sheetExpr`-keyed lookups, delta, the v1 wire types. The
      compiler's dead-code warnings are the checklist
- [ ] v1-only leftovers go too: `AppError::SnapshotRequired`, `Freshness`'s
      `catalogHash` doc, the `/api/v1` empty-404 fallback in `routes/mod.rs`
- [ ] CORS stops exposing `X-Total-Count` until a v2 route sends it
      (`community-charts` browse will)
- [ ] `test-foundation` 02's two deferred tests (the delta 409 path, the
      search pagination boundary) are closed as moot: their endpoints are gone
- [ ] `tests/snapshots/catalog.json` and the v1 snapshot test are deleted or
      replaced by 03's mapping check
- [ ] Contract: v1 sections removed; §7's wiring map describes v2
- [ ] `.sqlx/` regenerated; `--check` clean
- [ ] Spec *Done when* verified: no URL in the system carries a percent-encoded
      title
