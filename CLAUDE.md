# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Always read first

- [`docs/ROADMAP.md`](docs/ROADMAP.md) — what is being built now and what is deferred
- [`docs/architecture.md`](docs/architecture.md) — how the tiers fit together
- [`docs/adr/`](docs/adr/) — why decisions were made; check before proposing a change that contradicts one

Conditionally relevant:

- [`docs/reference/api-contract.md`](docs/reference/api-contract.md) — read before adding or changing any server endpoint
- [`docs/reference/schema.md`](docs/reference/schema.md) — read before touching tables or migrations
- `scripts/build-wasm.sh` — only when changing anything under `engine/` or the wasm-bindgen pin

## What this is

maiscope: a web app for browsing maimai songs and visualizing charts. Rust/TypeScript
monorepo — one Cargo workspace (`apps/server`, `engine`, `shared`) plus a
pnpm-managed frontend under `apps/host`.

The frontend is a port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)
(originally Nuxt 2 + Vuetify 2), rebuilt on native Vue 3 for maimai only.

## Commands

### Backend (`apps/server`)

```sh
docker compose up -d               # Postgres on :5432 — from the repo root
sqlx migrate run                   # the rest from apps/server
cargo run --bin sync_catalog       # fetch upstream and apply the diff
cargo run                          # API on :3000
cargo run --bin seed_songs         # load chart text, keyed by sheet_expr
```

`seed_songs` reads `songs/` at the repo root — a gitignored local sample, one
flat directory per song holding `maidata.txt`. It is absent on a fresh clone.
Matching is by the maidata `&title=`, so an unmatched title warns and skips.

#### Where infrastructure files live

**Root describes the system; `apps/<x>/` describes how one app is built.** The
dev database is shared by the server, `bin/sync_catalog` and the test suite, so
`docker-compose.yml` is at the root. The server image is one app's build, so
`apps/server/Dockerfile` sits with it.

Two deliberate exceptions, both forced by tooling:

- `.dockerignore` stays at the root — Docker resolves it against the build
  *context*, not against the Dockerfile
- `fly.toml` stays at the root — `flyctl` assumes it there, and its
  `[build] dockerfile` path resolves relative to it

The Cargo workspace spans the repo, so the build context is always the root:

```sh
docker build -f apps/server/Dockerfile -t maiscope-server:dev .
```

#### Offline sqlx cache

`.cargo/config.toml` sets `SQLX_OFFLINE=true` for the whole workspace, so builds
type-check the 64 `sqlx::query!` call sites against the committed `.sqlx/` cache
instead of a live Postgres. **Any change to SQL — including test fixture queries
inside `#[cfg(test)]` — breaks the next build until the cache is regenerated:**

```sh
# from the repo root, with Postgres up and migrations applied
set -a; . apps/server/.env; set +a
cargo sqlx prepare --workspace -- -p server --all-targets
```

Every flag matters. `--workspace` writes one cache at the repo root covering the
`bin/` tools. `-p server` restricts the underlying `cargo check` to the only crate
with query macros — omit it and the check compiles `engine` natively, which needs
Bevy's ALSA/X11/Wayland headers. `--all-targets` extends the check to test targets,
without which `cargo test` cannot compile offline. CI runs the same invocation with
`--check`, which exits non-zero on a stale or incomplete cache.

### Engine

```sh
./scripts/build-wasm.sh            # dev build; pass `release` for size-optimised
```

Output goes to `apps/host/src/wasm/` (gitignored). The `wasm-bindgen-cli` version
must match the crate version in `engine/Cargo.toml` **exactly** (pinned `=0.2.122`)
or bindgen fails on a schema mismatch.

### Frontend (`apps/host`)

```sh
pnpm install
pnpm dev                           # Vite dev server
pnpm build                         # vue-tsc --noEmit && vite build
```

### Workspace

```sh
cargo build --workspace            # excludes the engine's wasm target
cargo test --workspace
```

There is no lint or format config (no eslint/prettier). Do not assume `pnpm lint`
exists.

### Production operations (`.github/workflows/`)

Production is live: Fly.io `maiscope-api`, Neon Postgres, Cloudflare Pages. Every
production action is a workflow dispatch, never a local command.

| Workflow | Trigger | Effect |
|---|---|---|
| `ci.yml` | push / PR | checks named `rust`, `wasm-web`, `links` |
| `deploy.yml` | manual | Fly deploy; migrations run as the `release_command` |
| `deploy-web.yml` | manual | Cloudflare Pages build of `apps/host` |
| `sync-catalog.yml` | cron + manual | differential upstream sync; never deletes |
| `seed-charts.yml` | manual | chart text into production; insert/update only |
| `backup-database.yml` | cron + manual | `pg_dump` of Neon |

`deploy.yml` gates on the CI check-runs of the exact SHA and fails while any is
`pending`. That is the gate working — wait for CI and re-dispatch, never route
around it. `flyctl` is not installed locally; Fly-side settings are changed in
its web UI.

## Testing

Tests live per-module next to the code they cover, not centralized in
`main.rs`: `#[sqlx::test]` for anything touching the DB
(`apps/server/src/routes/*.rs`, `apps/server/src/queries/*.rs`,
`chart_revision.rs`), plain `#[test]`/`#[tokio::test]` for the rest
(`config.rs`, `error.rs`, `rate_limit.rs`, `bin/seed_songs.rs`). Middleware
behaviour is tested through an assembled `Router` via `tower::ServiceExt::oneshot`
(`routes/mod.rs`) — handler tests call handlers directly and never exercise a
layer.

`engine/` tests the simai parser. Per-note tests are inline `#[cfg(test)]`
modules in `systems/parser/` — `parse_note`, `parse_slide_note` and
`parse_duration_bracket` are private to that module tree, so an integration test
cannot reach them. Shared helpers live in `systems/parser/testutil.rs`. The
corpus test in `engine/tests/corpus.rs` drives the one public entry point,
`parse_chart`, over the fixtures in `engine/tests/fixtures/` and snapshots event
counts to `engine/tests/corpus-snapshot.txt`:

```sh
cargo test -p engine                              # needs libasound2-dev, libudev-dev
UPDATE_SNAPSHOT=1 cargo test -p engine --test corpus   # accept a counts change
```

`apps/host/` still has no tests. Both are tracked in
[`docs/work/test-foundation/`](docs/work/test-foundation/).

**Do not let `DATABASE_URL` point at a data-laden database when running tests.**
`#[sqlx::test]` derives each test's ephemeral database from the one `DATABASE_URL`
names, so its size is paid 45+ times over in parallel. Against a freshly migrated
database the suite is ~2.5s and green; against the dev database after a full
`seed_songs` (38MB, 6.5k charts) tests start failing with
`database "_sqlx_test_…" is being accessed by other users` as the teardown drops
race each other — a confusing failure that looks like broken code, not a bloated
template. Keep a lean database for tests:

```sh
docker exec maiscope-db psql -U postgres -c 'CREATE DATABASE maiscope_test'
DATABASE_URL=postgres://postgres:postgres@localhost:5432/maiscope_test cargo test --workspace
```

## Conventions

**Cross-tier key.** `sheetExpr = ${songId}|${type}|${difficulty}`. Frontend:
`utils/sheet.ts:computeSheetExpr`. Backend: the `sheet_expr` column. URL-encoded
in paths (`|` → `%7C`).

**Raw versus derived.** The server returns raw fields only; the frontend computes
`songNo`, `imageUrl`, `sheetExpr`, `notePercents`, `$canonicalSheet` in
`utils/data.ts:preprocessData`. Keep new endpoints on this split.

**Engine boundary.** JS↔Bevy goes through the mailbox in `engine/src/wasm_bridge.rs`,
drained once per frame by `apply_commands`. Never call into Bevy synchronously.

## Rules

- **Never reach production from this machine.** Local work runs against dev
  Postgres via `docker compose` and local wasm builds. Production is reached only
  by dispatching a workflow (see *Production operations*) — no `flyctl`, no
  `psql`, no `pg_dump` against a Neon URL, no secrets pasted into a shell. If a
  task needs a production change, name the workflow and let the user dispatch it.
- **Doc sync is part of the ticket that changes behaviour**, not a trailing ticket.
  Changing an endpoint means updating `docs/reference/api-contract.md` in the same
  change; changing the schema means `docs/reference/schema.md`.
- **Record decisions as ADRs.** A choice with a rejected alternative belongs in
  `docs/adr/`, not buried in a spec.

## Working with the maintainer

This is a solo learning project. The code is a vehicle for learning Rust, Axum
and deployment, so *how* work happens matters as much as the result.

- **"Instruct me to do task N" means do not write the code.** Lay out the steps,
  the signatures and the reasoning, then stop and let them implement. "Help me
  with task N" is the opposite — write it. Honour the verb they used.
- **Explanations worth keeping go to `.claude/study/<topic>.md`** (gitignored),
  one file per concept, written after the code lands — existing notes cover
  `tracing-subscriber`, `env-filter`, rate limiting, CORS.
- **One ticket at a time, committed on its own.** Per-ticket review, then one
  whole-branch review before merging to master.
- Decisions are made by picking from options. Offer a small labelled set with
  trade-offs and a recommendation rather than one pre-chosen path.

## Agent skills

**Issue tracker.** Work lives as markdown under `docs/work/<feature-slug>/`.
See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

**Domain docs.** See [`docs/agents/domain.md`](docs/agents/domain.md).
