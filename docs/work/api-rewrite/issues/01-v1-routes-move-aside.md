# 01 — v1 routes move under `routes/v1/`

**What to build:** move the v1 route modules (`catalog`, `charts`, `health`,
`sheets`, `songs`, `sync`, and `caching`'s v1-only parts if any) into
`apps/server/src/routes/v1/`, so the v2 modules can take the plain names in
`routes/`. No behaviour changes: `/api/v1/...` answers byte-for-byte as before.

The alternative, v2 under a new `src/api/` tree, leaves the auth routes (already
v2, in `routes/auth.rs`) on the wrong side and makes ticket 07 a rename of
everything that stays. Moving v1 aside makes ticket 07 a single `rm -r`.

**Blocked by:** None.

**Status:** done

- [x] `routes/v1/mod.rs` exposes the v1 routes — as `health_router()` and
      `limited_router()`, see below — for `routes/mod.rs` to nest at `/api/v1`, with the same rate limiter and the health exemption intact
- [x] Shared pieces stay outside `v1/`: `caching::Freshness` (the v2 Catalog and
      manifest need it), `AppError`, state, rate limiting
- [x] Every existing test still passes, unmoved in meaning; the router tests in
      `routes/mod.rs` keep exercising `/api/v1` paths until 07
- [x] `CLAUDE.md`'s testing section names `routes/v1/` where it lists route tests

## Comments

**2026-11-17 — built.** `git mv`, so each file's history follows it.
`routes/v1/mod.rs` exposes `health_router()` and `limited_router()` rather
than one router, so the caller still decides what goes behind the limiter
and the health exemption stays where its router test looks for it. `caching`
stays in `routes/`, shared with v2. 215 server tests pass unchanged.

**2026-11-17 — review.** Applied: stale paths in the api-rewrite spec,
`community-charts` 04, ADR-0010 (as a parenthetical, it is a record), the
contract's status legend and two code comments. Historical plans and closed
tickets under `docs/work/` keep the paths they were written against.
