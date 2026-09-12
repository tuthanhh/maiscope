# 04 — CI: pull-request checks

**What to build:** The repo has no CI at all. Add one workflow that runs on every
PR and gates merge.

Note the existing test surface is smaller than CLAUDE.md claims: six
`#[sqlx::test]` handler tests (`apps/server/src/main.rs:484+`), a scaffold stub in
`shared`, and nothing in `engine` or `apps/host`. `test-foundation` issues 01–02 grow it; this
ticket just makes what exists run automatically.

**Blocked by:** 01

**Status:** todo

- [ ] `.github/workflows/ci.yml`, triggered on `pull_request` and pushes to master
- [ ] Postgres service container, migrations applied before tests
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace -- -D warnings` (expect an initial cleanup pass)
- [ ] `cargo sqlx prepare --check` — catches a changed query with a stale `.sqlx/`
- [ ] `cargo test --workspace`
- [ ] `cargo build --target wasm32-unknown-unknown -p engine` — catches wasm-only
      breakage without running `wasm-bindgen`
- [ ] `pnpm install --frozen-lockfile && pnpm build` (`vue-tsc --noEmit` + build)
- [ ] Rust and pnpm caching so runs stay under a few minutes
- [ ] Branch protection: these checks required before merge

## Comments

**Pre-existing failures this ticket inherits**, measured while closing issue 01.
Both need a cleanup commit *before* the workflow is added, or the first CI run is
red for reasons unrelated to CI:

- `cargo fmt --check` — exits 1, **57 hunks across 14 files**, heaviest in
  `queries/sheets.rs`, `chart_revision.rs`, and `routes/catalog.rs`. One
  mechanical `cargo fmt` commit, kept separate from anything else.
- `cargo clippy -- -D warnings` — `apps/server/src/error.rs:21`, `variant
  BadRequest is never constructed`. Either construct it, `#[allow(dead_code)]` it,
  or drop the variant.

**`sqlx prepare --check` must carry issue 01's flags exactly:**

```sh
cargo sqlx prepare --check --workspace -- --all-targets
```

Without `-- --all-targets` the check compiles fewer targets than the committed
cache covers and disagrees with it. Verified to exit 1 on an incomplete cache and
0 on a clean one.

**`cargo test` still needs the Postgres service**, even though `.cargo/config.toml`
sets `SQLX_OFFLINE=true`. Offline mode removes the database from the *compile*
step only; `#[sqlx::test]` creates a throwaway database per test at runtime and
requires a reachable `DATABASE_URL`.
