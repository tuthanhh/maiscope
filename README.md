# maiscope

A desktop song browser + real-time chart visualizer for **maimai** (SEGA's arcade
rhythm game), built as a Rust/TypeScript monorepo: a Tauri desktop frontend, a
Rust backend serving a community chart database, and a Bevy game engine
compiled to WebAssembly for in-app chart playback.

## Architecture

Three tiers, one Cargo workspace:

```
apps/host/     Tauri + Vite + Vue 3 desktop app — song browser UI, embeds the
                engine's wasm build on its visualizer page
apps/server/   Rust/Axum + Postgres — global chart catalog & per-sheet chart API
engine/        Bevy ECS chart-rendering engine, compiled to wasm and imported
                by apps/host (native Vue3, no server-side rendering)
shared/        Rust crate for types shared across the workspace (scaffolded,
                not yet wired into server/host — see Roadmap)
```

```
apps/host (browser UI)              apps/server (Rust/Axum/Postgres)
   │  GET /api/v1/catalog  ────────────▶  song/sheet metadata (Postgres)
   │  GET /api/v1/sheets/{s}/chart ────▶  raw simai chart text
   ▼
visualizer page
   │  load_chart(text) via wasm-bindgen
   ▼
engine (Bevy, compiled to wasm32-unknown-unknown)
   parses simai → spawns notes → renders synced to the BGM clock
```

Frontend catalog browsing (list/grid, filters, i18n, sheet details) is a
desktop port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)
(originally Nuxt2/Vuetify2), rebuilt on native Vue 3 for a single game
(maimai only — the former multi-game `/:gameCode` routing was removed). The
visualizer engine started as an independent Bevy learning project and is now
embedded as the chart-rendering core.

## Getting started

**Prerequisites**
- [Rust toolchain](https://rustup.rs/) (stable, edition 2024) + `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` **0.2.122** exactly (must match `engine/Cargo.toml` — mismatched versions fail bindgen with a schema error)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- Docker (local Postgres) + [`sqlx-cli`](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) for migrations
- [Tauri system prerequisites](https://tauri.app/start/prerequisites/) for your OS (only needed for the native desktop build)

**1. Backend — Postgres + server**
```sh
cd apps/server
cp .env.example .env               # adjust if needed; must match docker-compose.yml
docker compose up -d               # starts Postgres on :5432
sqlx migrate run
cargo run --bin ingest             # pulls upstream data.json into canonical tables
cargo run                          # starts the API on :3000
```

**2. Engine — build the wasm chart viewer**
```sh
./engine/build-wasm.sh             # dev build (fast, ~74MB); pass `release` for a size-optimized build
```
Output lands in `apps/host/src/wasm/` (gitignored — rebuild after pulling changes to `engine/`).

**3. Frontend — desktop app**
```sh
cd apps/host
pnpm install
pnpm dev            # browser dev server (Vite)
# or
pnpm tauri dev       # native desktop window
```
Set `VITE_API_BASE_URL` if the server isn't at the default `http://localhost:3000/api/v1`.

## Features

- Song gallery with search, filtering (level, category, version, BPM, region…), list/grid views
- Per-sheet detail dialog (difficulty, internal level, note designer, note counts)
- Real-time chart visualizer: taps, holds, touch notes, slides (star + path traces), BGM-synced clock
- Light/dark mode, multi-language UI (EN, JA, KO, ZH-Hans, ZH-Hant, VI, ES, ID, RU)
- Native desktop shell via Tauri 2 (with a plain-browser fallback)

## Roadmap

- [x] Backend catalog (`GET /catalog`) wired to the frontend, replacing the legacy CloudFront `data.json` fetch
- [ ] Wire the `shared` crate as the single source of truth for API contract types (server ↔ generated frontend types)
- [ ] Local SQLite cache for offline-first browsing on desktop
- [ ] GitHub login + community chart contribution flow (auth, submission UI)
- [ ] Production hardening: test coverage, CI, server observability/rate-limiting, release wasm pipeline

See `apps/server/docs/api-contract.md` and `apps/server/docs/schema.md` for the backend contract and schema.

## Acknowledgments

- **[zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)** — the original web app this frontend is ported from. Data model, filtering logic, and UI design credit goes to the upstream project.
- [mai-notes.com](https://mai-notes.com/) and [majdata.net](https://majdata.net/) — simai chart format and note-timing references
- [maimai でらっくす 公式サイト｜セガ](https://maimai.sega.jp/) — official song information

## License

MIT — see [LICENSE](./LICENSE).
