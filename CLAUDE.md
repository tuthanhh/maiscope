# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Always read first

- root `README.md` -- project overview + roadmap; trust this over `apps/host/README.md` (stale, pre-migration)
- `apps/server/docs/api-contract.md` -- full HTTP contract, read before adding/changing any server endpoint
- `apps/server/docs/schema.md` -- Postgres schema, read before touching DB tables or migrations

Conditionally relevant:
- `engine/build-wasm.sh` -- only if changing anything under `engine/` or the wasm-bindgen pin
- `apps/host/src/app/game.ts` -- only if changing how the frontend talks to the server (`VITE_API_BASE_URL`)

## What this is

maiscope: desktop song browser + real-time chart visualizer for **maimai** (SEGA arcade rhythm game). Rust/TypeScript monorepo, single Cargo workspace (`Cargo.toml` members: `apps/server`, `apps/host/src-tauri`, `engine`, `shared`) plus a pnpm-managed frontend under `apps/host`.

```
apps/host/     Tauri + Vite + Vue 3 desktop app — song browser UI, embeds engine's wasm build on its visualizer page
apps/server/   Rust/Axum + Postgres — global chart catalog & per-sheet chart API
engine/        Bevy ECS chart-rendering engine, compiled to wasm32-unknown-unknown, imported by apps/host
shared/        Scaffolded Rust crate for cross-workspace types — NOT yet wired into server/host (roadmap item)
```

Data flow: `apps/host` fetches catalog/chart JSON from `apps/server` (`/api/v1/...`) → visualizer page hands raw simai text + audio bytes into the `engine` wasm module via `wasm-bindgen` → Bevy parses simai, spawns notes, renders synced to the BGM clock.

Frontend catalog browsing (list/grid, filters, i18n, sheet details) is a desktop port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs) (originally Nuxt2/Vuetify2), rebuilt on native Vue 3, single game only (maimai — former multi-game `/:gameCode` routing removed). **`apps/host/README.md` is a stale pre-migration doc** (mentions Vuetify/echarts/multi-game that no longer exist) — trust the root `README.md` and this file instead.

## Commands

### Backend (`apps/server`)
```sh
cd apps/server
docker compose up -d               # Postgres on :5432 (creds in .env, copy from .env.example)
sqlx migrate run                   # apply migrations
cargo run --bin ingest             # pull upstream data.json into canonical tables
cargo run                          # start API on :3000
cargo run --bin seed_chart         # seed a chart row for local testing
```

### Engine wasm (`engine/`)
```sh
./engine/build-wasm.sh             # dev build (~74MB); pass `release` for size-optimized
```
Builds via the root workspace (`cargo build --target wasm32-unknown-unknown`, artifacts land in root `target/`, not `engine/target/`), then runs `wasm-bindgen`, output to `apps/host/src/wasm/` (gitignored — rebuild after any `engine/` change) and syncs sprite assets to `apps/host/public/assets/sprites`.

**Gotcha**: `wasm-bindgen-cli` version must match the `wasm-bindgen` crate version in `engine/Cargo.toml` (currently pinned `=0.2.122`) *exactly*, or bindgen fails with a schema-version mismatch.

### Frontend (`apps/host`)
```sh
cd apps/host
pnpm install
pnpm dev             # Vite dev server (browser)
pnpm tauri dev        # native desktop window
pnpm build            # vue-tsc --noEmit && vite build
```
Set `VITE_API_BASE_URL` if the server isn't at the default `http://localhost:3000/api/v1` (see `apps/host/src/app/game.ts`).

### Rust workspace-wide
```sh
cargo build --workspace         # excludes engine's wasm target, which builds separately
cargo test -p shared            # only crate with tests currently (scaffold stub)
```

No CI, no lint/format config (eslint/prettier), and no test suite beyond `shared`'s scaffold — don't assume `pnpm lint` or similar exist.

## Architecture notes

### `apps/server` — global backend
- Axum handlers live in `apps/server/src/main.rs`; DB row/JSON types in `types.rs`.
- Postgres schema: canonical catalog tables (built) + charts/assets/contributions (later). See `apps/server/docs/schema.md`.
- **Cross-tier identity key**: `sheetExpr = ${songId}|${type}|${difficulty}` (frontend: `utils/sheet.ts:computeSheetExpr`; backend: `sheet_expr` column). Almost every per-sheet endpoint keys on this, URL-encoded (`|` → `%7C`).
- Server returns **raw fields only** — it never sends client-derived fields (`songNo`, `imageUrl`, `sheetExpr`, `notePercents`, `$canonicalSheet`); the frontend computes those in `utils/data.ts:preprocessData`. Keep new endpoints consistent with this split.
- Full HTTP contract (including not-yet-built auth/contributions/sync tiers) is in `apps/server/docs/api-contract.md` — read it before adding or changing an endpoint.
- Single game only: no `games` table, no `game_code` anywhere in the schema.

### `engine` — Bevy chart visualizer
- `engine/src/lib.rs` is the wasm entry point (`#[wasm_bindgen(start)]`); auto-boots the Bevy `App` on module init and grabs `<canvas id="bevy">`, which must already exist in the DOM.
- `engine/src/wasm_bridge.rs` is the JS↔Bevy boundary: a thin lock-guarded mailbox pattern — JS pushes `SongPayload`/`EngineCommand` into a `Mutex<Vec<_>>` "inbox", a Bevy system (`apply_commands`, in `engine/src/systems/visual/spawning.rs`) drains it once per frame. Follow this pattern for any new engine↔JS command rather than calling into Bevy synchronously.
- `engine/src/systems/parser/` parses simai chart text; `engine/src/systems/visual/` spawns/renders notes (taps, holds, touch, slides); `engine/src/systems/chart_playback.rs` drives the BGM-synced clock; `engine/src/plugins/` wires Bevy plugins (audio via `bevy_kira_audio`, camera, shape rendering via `bevy_prototype_lyon`).
- Only sprite textures go through Bevy's `AssetServer` (fetched at runtime from `/assets/...`, mirrored into `apps/host/public/assets/sprites` by `build-wasm.sh`). Chart text and audio bytes are pushed in directly via `wasm_bridge`, not file-fetched.
- Native (non-wasm) build also works via `engine/src/main.rs` — useful for developing/debugging Bevy systems without the wasm round-trip.

### `apps/host` — frontend
- Path alias `~` → `apps/host/src` (see `vite.config.ts` / `tsconfig.json`).
- `composables/useEngine.ts` is the sole bridge into the wasm module: lazily loads `~/wasm/maiscope_viewer.js` (import is dynamic — never bundled into the main chunk), owns a single persistent `<canvas id="bevy">` that's reparented on page mount/unmount rather than recreated (Bevy can't re-attach to a new canvas element once initialized).
- `stores/data.ts` + `utils/data.ts:preprocessData` rehydrate the catalog JSON into a frozen, prototype-linked object graph — this is inherited from the upstream arcade-songs data model; derived fields (`imageUrl`, `sheetExpr`, `notePercents`, etc.) are computed client-side here, never sent by the server.
- `src-tauri/` is the native shell; it also proxies authenticated writes through Rust (`reqwest`) to dodge browser CORS for the future contribution flow — see api-contract.md §5/§6.
- Locale files live in `locales/*.yaml`, loaded via `@modyfi/vite-plugin-yaml`; supported: en, ja, ko, zh-Hans, zh-Hant, vi, es, id, ru.

## Roadmap context (affects design decisions)

- `shared` crate is scaffolded but unused — don't assume it's wired into server/host request/response types yet; that's a planned future step (single source of truth for the API contract). (`shared/src/lib.rs` still holds the default `cargo new` scaffold — a placeholder `add()` fn and its test — no real types defined yet.)
- Local SQLite cache (offline-first), GitHub-auth contribution flow, and CI/test coverage are all unbuilt — see root `README.md` Roadmap and `apps/server/docs/api-contract.md` §3–§5 before assuming these exist.

## Important rules

- **Never touch production.** Everything here is local (dev Postgres via `docker compose`, local wasm builds, local Tauri dev). No production DB, deploy target, or hosted environment exists in this workflow — if a task implies one, stop and ask rather than acting.
- **Doc sync**: when adding or changing a server endpoint, update `apps/server/docs/api-contract.md` (and `apps/server/docs/schema.md` if the schema changed) in the same change — these are the source of truth for the HTTP contract, not the handler code alone.

## Agent skills

### Issue tracker

Issues tracked as local markdown under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context layout — root `CONTEXT.md` + `docs/adr/` (created lazily by domain-modeling skill). See `docs/agents/domain.md`.
