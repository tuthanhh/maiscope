# 01 — `cargo sqlx prepare`: commit `.sqlx/`, build with `SQLX_OFFLINE`

**What to build:** The server uses **62 compile-time `sqlx::query!`/`query_as!`
macros** and there is no `.sqlx/` directory anywhere in the repo. So `cargo build`
currently **requires a reachable `DATABASE_URL`** — which a `docker build` does
not have. This blocks containerisation outright.

Generate the offline query cache, commit it, and build with `SQLX_OFFLINE=true`.
CI runs `cargo sqlx prepare --check` against a service Postgres so that changing
a query without re-preparing fails the build with a clear message instead of at
runtime.

**Blocked by:** `server-restructure/issues/04-split-modules.md` (do it once the SQL has finished moving between modules)

**Status:** done

- [x] `cargo sqlx prepare --workspace` run against a migrated local DB
- [x] `.sqlx/` committed (it is generated state, and that is intentional)
- [ ] `SQLX_OFFLINE=true` set for the Docker build — deferred to issue 02, but
      already satisfied repo-wide by `.cargo/config.toml`, so the builder stage
      inherits it from the source tree
- [x] Local dev builds offline by default — `.cargo/config.toml` sets
      `SQLX_OFFLINE=true` for the workspace (see Comments for why, over the
      alternative of leaving it unset)
- [ ] `cargo sqlx prepare --check` wired into CI — deferred to issue 04; verified
      by hand to exit 1 on an incomplete cache
- [x] Contributor note in CLAUDE.md: re-run `prepare` after any query change

## Comments

**Macro count and cache size.** 64 real call sites (34 `query!`, 16 `query_as!`,
14 `query_scalar!`) — the spec's "62" counted two occurrences that are prose in
backticks. These collapse to **43** cache files, because entries are keyed by a
hash of the SQL string and identical SQL at several call sites dedupes to one
file.

**`-- --all-targets` is required.** A plain `cargo sqlx prepare --workspace`
produced only 24 files and `cargo test --no-run` then failed offline: `prepare`
caches only what its internal `cargo check` compiles, which excludes
`#[cfg(test)]` code. The uncached queries were all test fixture inserts
(`queries/catalog.rs:66`, `queries/sheets.rs:414+`). Since `sqlx-cli` forwards
arguments after `--` straight to `cargo check`, the fix is:

```sh
cargo sqlx prepare --workspace -- --all-targets
```

**Issue 04 must use the same flags.** `prepare --check` compares against whatever
the flags cause to be compiled, so a `--check` without `-- --all-targets` would
disagree with this cache.

**Decision: `SQLX_OFFLINE=true` in `.cargo/config.toml`, not left unset.**
Rejected alternative was leaving it unset locally, which is the conventional sqlx
default and keeps the inner loop frictionless — a stale cache would then only be
caught by CI. Chosen because spec.md already commits to "a stale cache is a build
failure with a clear message instead of a runtime surprise"; applying that locally
moves the failure from a PR to the next `cargo build`. Cost accepted: every query
change now needs Postgres up and a re-`prepare` before anything compiles.

The `[env]` entry is deliberately not `force = true`, so an explicit variable
still wins. In practice no override is needed — `sqlx-cli` passes
`SQLX_OFFLINE=false` to its child `cargo check`, and cargo's `[env]` does not
override an inherited variable, so `prepare` regenerates correctly with the config
in place (verified by deleting a cache entry and regenerating it).

**Verification.** With no environment variables set at all, `cargo build -p server
--bins` and `cargo test -p server --no-run` both succeed, and moving `.sqlx/` aside
makes them fail with `no cached data for this query` — confirming the build reads
the cache rather than silently connecting via `apps/server/.env`.

**Landmine for issue 04.** `apps/server/src/error.rs:21` warns `variant
BadRequest is never constructed`, which fails that ticket's
`cargo clippy --workspace -- -D warnings`.
