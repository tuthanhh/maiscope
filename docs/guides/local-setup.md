# Get maiscope running locally

## Prerequisites

- [Rust toolchain](https://rustup.rs/), pinned to `1.97` by `rust-toolchain.toml`
  at the repo root — which also declares the `wasm32-unknown-unknown` target, so
  `rustup show` installs everything the engine build needs
- `wasm-bindgen-cli` **0.2.122 exactly** — it must match the `wasm-bindgen` crate
  version pinned in `engine/Cargo.toml`, or bindgen fails with a schema-version
  mismatch
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- Docker (for local Postgres) and [`sqlx-cli`](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)

## 1. Backend

```sh
docker compose up -d          # Postgres on :5432, from the repo root
cd apps/server
cp .env.example .env          # values must match the root docker-compose.yml
sqlx migrate run              # reads migrations/ relative to apps/server
cargo run --bin sync_catalog  # fetch upstream and apply the diff
cargo run                     # API on :3000
```

The database is declared at the repo root because it is shared — sync_catalog,
the tests and the server all use it. `sqlx` commands stay in `apps/server`,
which is where `migrations/` and `.env` live.

## 2. Engine

```sh
./scripts/build-wasm.sh       # dev build; pass `release` for a size-optimised one
```

Output lands in `apps/host/src/wasm/` (gitignored — rebuild after any change under
`engine/`). Artifacts build through the root workspace, so they appear in the root
`target/`, not `engine/target/`.

This step is not optional for the frontend. `apps/host/src/composables/useEngine.ts`
imports `~/wasm/engine.js` at type level, so `pnpm build` — which runs
`vue-tsc --noEmit` first — fails without it, and the visualizer route fails at
runtime. Skipping it leaves the rest of the app working.

## 3. Frontend

```sh
cd apps/host
pnpm install
pnpm dev                      # Vite dev server
```

Set `VITE_API_BASE_URL` if the server is not at the default
`http://localhost:3000/api/v1` — see `apps/host/src/app/game.ts`.

## Troubleshooting

**Blank canvas on the visualizer page.** The Bevy app attaches to
`<canvas id="bevy">` on module init; the element must exist in the DOM first.

**`wasm-bindgen` schema error.** The CLI and crate versions differ. Check
`engine/Cargo.toml` and install the matching CLI version.
