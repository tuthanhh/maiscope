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

**Pre-existing failures this ticket inherited — now cleared** (`eddf006`,
`831f225`), so CI's first run can be red only for things CI actually catches:

- `cargo fmt --check` exited 1 on **57 hunks across 14 files**. Fixed by one
  mechanical `cargo fmt` commit, deliberately isolated so it does not bury the
  lint fixes.
- `cargo clippy --workspace --all-targets -- -D warnings` failed on **four**
  lints, not the one spotted from `apps/server` alone — running clippy across the
  workspace surfaced two more in `engine`. Two were silenced with `#[allow]`
  (`AppError::BadRequest`, `SlideElement::is_break`, `NoteKind::SlideStar` as
  deliberately-unused API surface; `too_many_arguments` on `spawn_note`), two
  fixed properly (`collapsible_if` → Rust 2024 let-chain in `seed_songs.rs`;
  `get(&k).is_none()` → `!contains_key(&k)` in `sheets.rs`; loop-counter indexing
  of `COUNTDOWN_EDGE_COLORS` → `.iter().enumerate()`).

Both gates now exit 0.

**Correction to this ticket's premise.** The body says the test surface is "six
`#[sqlx::test]` handler tests … and nothing in `engine` or `apps/host`", and cites
`apps/server/src/main.rs:484+`, which no longer holds after `server-restructure`
04 split the modules. Actual count as of 2026-09-13: **69 tests across 11 binaries**,
all passing. Still nothing in `apps/host`.

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
