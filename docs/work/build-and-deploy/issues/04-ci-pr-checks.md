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
