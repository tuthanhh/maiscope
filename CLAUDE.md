# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Always read first

- [`docs/ROADMAP.md`](docs/ROADMAP.md) — what is being built now and what is deferred
- [`docs/architecture.md`](docs/architecture.md) — how the tiers fit together
- [`docs/adr/`](docs/adr/) — why decisions were made; check before proposing a change that contradicts one

Conditionally relevant:

- [`docs/reference/api-contract.md`](docs/reference/api-contract.md) — read before adding or changing any server endpoint
- [`docs/reference/schema.md`](docs/reference/schema.md) — read before touching tables or migrations
- `engine/build-wasm.sh` — only when changing anything under `engine/` or the wasm-bindgen pin

## What this is

maiscope: a web app for browsing maimai songs and visualizing charts. Rust/TypeScript
monorepo — one Cargo workspace (`apps/server`, `engine`, `shared`) plus a
pnpm-managed frontend under `apps/host`.

The frontend is a port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)
(originally Nuxt 2 + Vuetify 2), rebuilt on native Vue 3 for maimai only.

## Commands

### Backend (`apps/server`)

```sh
docker compose up -d               # Postgres on :5432
sqlx migrate run
cargo run --bin ingest             # load catalog into canonical tables
cargo run                          # API on :3000
cargo run --bin seed_songs         # load chart text, keyed by sheet_expr
```

### Engine

```sh
./engine/build-wasm.sh             # dev build; pass `release` for size-optimised
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

## Testing

Six `#[sqlx::test]` handler tests live in `apps/server/src/main.rs`. `engine/` and
`apps/host/` have no tests yet — growing that is tracked in
[`docs/work/production-v1/`](docs/work/production-v1/) issues 26 and 27.

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

- **Never touch production.** Everything local: dev Postgres via `docker compose`,
  local wasm builds. If a task implies a production system, stop and ask.
- **Doc sync is part of the ticket that changes behaviour**, not a trailing ticket.
  Changing an endpoint means updating `docs/reference/api-contract.md` in the same
  change; changing the schema means `docs/reference/schema.md`.
- **Record decisions as ADRs.** A choice with a rejected alternative belongs in
  `docs/adr/`, not buried in a spec.

## Agent skills

**Issue tracker.** Work lives as markdown under `docs/work/<feature-slug>/`.
See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

**Domain docs.** See [`docs/agents/domain.md`](docs/agents/domain.md).
