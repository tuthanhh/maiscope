# 04 — CI: pull-request checks

**What to build:** The repo has no CI at all. Add one workflow that runs on every
PR and gates merge.

Note the existing test surface is smaller than CLAUDE.md claims: six
`#[sqlx::test]` handler tests (`apps/server/src/main.rs:484+`), a scaffold stub in
`shared`, and nothing in `engine` or `apps/host`. `test-foundation` issues 01–02 grow it; this
ticket just makes what exists run automatically.

**Blocked by:** 01

**Status:** in-progress

- [x] `.github/workflows/ci.yml`, triggered on `pull_request` and pushes to master
- [x] Postgres service container (`postgres:17`, matching `docker-compose.yml`),
      migrations applied before the checks that need them
- [x] `cargo fmt --check`
- [x] clippy under `-D warnings` — **scoped**, see Comments:
      `-p server -p shared --all-targets` natively, plus
      `-p maiscope-viewer --target wasm32-unknown-unknown`
- [x] `cargo sqlx prepare --check --workspace -- -p server --all-targets`
- [x] `cargo test -p server -p shared` — **scoped**, see Comments. Covers all 69
      tests; `engine` has none
- [x] wasm breakage caught — by the engine clippy run above rather than a separate
      `cargo build --target …`. Clippy compiles, so the build step was redundant
- [x] `pnpm install --frozen-lockfile && pnpm build`
- [x] Rust (`Swatinem/rust-cache`, one key per job) and pnpm
      (`setup-node cache: pnpm`) caching
- [ ] Verified on a real PR — every job green, timings recorded
- [ ] Branch protection: these checks required before merge (GitHub UI; check
      names only appear after the workflow has run once)

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

**Two jobs, not three, and clippy/test scoped to `server` + `shared`.**

Measured before deciding: `server` resolves 229 crates, `engine` 309; `engine` has
**0 tests**. So `--workspace` would add ~300 crates, a multi-gigabyte `target/`
cache against a 10GB per-repo Actions budget, and an apt install of
`libasound2-dev libudev-dev libx11-dev libxkbcommon-dev libwayland-dev pkg-config`
(`bevy_kira_audio` → cpal → ALSA, plus the `x11`/`wayland` features) — all to lint
a native build of a crate that only ever ships as wasm, and to run zero tests.

Instead `engine` is gated by
`cargo clippy -p maiscope-viewer --target wasm32-unknown-unknown -- -D warnings`,
which needs no system libraries (ALSA/X11/Wayland are `cfg`-ed out on `wasm32`),
lints the target that actually ships, and — since clippy compiles — subsumes the
separate `cargo build --target …` step. Verified locally: passes clean in 45s.

Rejected: `--workspace`, for the costs above. The trade accepted is that
`engine`'s native-only code paths go unlinted; revisit if a native viewer build
ever becomes real. Note the package is named **`maiscope-viewer`**, not `engine`.

**The frontend cannot be its own job.** `pnpm build` fails without the wasm
bindings — `useEngine.ts:13,67` import `~/wasm/maiscope_viewer.js`, and
`apps/host/src/wasm/` is gitignored, so a bare checkout gives two `TS2307` errors.
The engine and frontend are therefore one `wasm-web` job: clippy → `build-wasm.sh`
→ `pnpm build`. Keeping them together also lets clippy's dependency rlibs feed the
wasm build from the same `target/`, instead of passing a ~140MB artifact between
jobs.

**Pins added, because CI is where unpinned versions bite.** `rust-toolchain.toml`
(channel 1.97, `rustfmt`/`clippy`, `wasm32-unknown-unknown`) so one file drives
local, CI and `apps/server/Dockerfile`; CI just runs `rustup show`. And
`packageManager: pnpm@10.33.0` in `apps/host/package.json`, which
`pnpm/action-setup` reads via `package_json_file` — there is no root
`package.json` for it to find.

**No `paths:` filter**, unlike `docs.yml`. A required check skipped by a path
filter never reports, and GitHub blocks the merge waiting for it.

**`sqlx prepare --check` must carry issue 01's flags exactly:**

```sh
cargo sqlx prepare --check --workspace -- -p server --all-targets
```

Without `-- --all-targets` the check compiles fewer targets than the committed
cache covers and disagrees with it. Verified to exit 1 on an incomplete cache and
0 on a clean one.

**`cargo test` still needs the Postgres service**, even though `.cargo/config.toml`
sets `SQLX_OFFLINE=true`. Offline mode removes the database from the *compile*
step only; `#[sqlx::test]` creates a throwaway database per test at runtime and
requires a reachable `DATABASE_URL`.
