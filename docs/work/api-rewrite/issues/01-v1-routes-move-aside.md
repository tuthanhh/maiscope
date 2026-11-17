# 01 — v1 routes move under `routes/v1/`

**What to build:** move the v1 route modules (`catalog`, `charts`, `health`,
`sheets`, `songs`, `sync`, and `caching`'s v1-only parts if any) into
`apps/server/src/routes/v1/`, so the v2 modules can take the plain names in
`routes/`. No behaviour changes: `/api/v1/...` answers byte-for-byte as before.

The alternative, v2 under a new `src/api/` tree, leaves the auth routes (already
v2, in `routes/auth.rs`) on the wrong side and makes ticket 07 a rename of
everything that stays. Moving v1 aside makes ticket 07 a single `rm -r`.

**Blocked by:** None.

**Status:** todo

- [ ] `routes/v1/mod.rs` exposes one `router()` that `routes/mod.rs` nests at
      `/api/v1`, with the same rate limiter and the health exemption intact
- [ ] Shared pieces stay outside `v1/`: `caching::Freshness` (the v2 Catalog and
      manifest need it), `AppError`, state, rate limiting
- [ ] Every existing test still passes, unmoved in meaning; the router tests in
      `routes/mod.rs` keep exercising `/api/v1` paths until 07
- [ ] `CLAUDE.md`'s testing section names `routes/v1/` where it lists route tests
