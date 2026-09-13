# Architecture

How the pieces fit together. Decisions and their rationale live in [`adr/`](adr/);
this document describes the result.

## Tiers

```
apps/host/     Vue 3 + Vite web app, PWA-installable — song browser and visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm32-unknown-unknown
shared/        Scaffolded Rust crate for cross-workspace types — not yet wired in
```

One Cargo workspace (`apps/server`, `engine`, `shared`) plus a pnpm-managed
frontend under `apps/host`.

## Data flow

```
apps/host (browser)                    apps/server (Fly.io)
   │  GET /api/v1/catalog       ──────▶  song/sheet metadata (Neon Postgres)
   │  GET /api/v1/sheets/{s}/chart ───▶  raw simai chart text
   ▼
visualizer page
   │  load_chart(text) via wasm-bindgen
   ▼
engine (Bevy, wasm)
   parses simai → spawns notes → renders on a wall-clock-driven chart clock
```

## Cross-tier identity

`sheetExpr` = `` `${songId}|${type}|${difficulty}` ``. Computed client-side in
`apps/host/src/utils/sheet.ts:computeSheetExpr`; stored as the `sheet_expr` column
server-side. Almost every per-sheet endpoint keys on it, URL-encoded (`|` → `%7C`).

## Raw versus derived fields

The server returns **raw fields only**. It never sends `songNo`, `imageUrl`,
`imageUrlM`, `sheetExpr`, `notePercents`, or `$canonicalSheet` — the frontend
computes those in `apps/host/src/utils/data.ts:preprocessData`, which rehydrates
the catalog JSON into a frozen, prototype-linked object graph inherited from the
upstream arcade-songs data model. Keep new endpoints on this split.

## Client caching

Three layers with three lifetimes. Knowing which is which is the difference
between debugging staleness in one minute and three.

| Layer | Holds | Invalidated by |
|---|---|---|
| Service worker | App shell (precached); wasm (runtime, cache-first) | Content-hashed filenames; `index.html` is network-first |
| IndexedDB | Catalog blob (v1.1) | `GET /sync/manifest` revision change; paint-then-swap |
| HTTP | Wire responses | `ETag` / `If-None-Match` / `Cache-Control` |

## Engine boundary

`engine/src/lib.rs` is the wasm entry point (`#[wasm_bindgen(start)]`); the Bevy
app boots on module init and attaches to `<canvas id="bevy">`, which must already
be in the DOM.

`engine/src/wasm_bridge.rs` is the JS↔Bevy boundary: a lock-guarded mailbox. JS
pushes `SongPayload` / `EngineCommand` into a `Mutex<Vec<_>>`; the `apply_commands`
system (`engine/src/systems/visual/spawning.rs`) drains it once per frame. Add new
engine commands this way — never call into Bevy synchronously.

The chart clock has two modes. `load_chart(text)` advances on the wall clock
(`ChartPlayback::advance`); `load_song(text, bytes)` slaves the clock to the BGM
(`ChartPlayback::sync_to_audio_position`). v1 uses the silent path only — see
[ADR-0002](adr/0002-chart-data-only-no-audio-hosting.md).

Only sprite textures go through Bevy's `AssetServer`, fetched at runtime from
`/assets/...`. Chart text is pushed in directly through `wasm_bridge`.

## Deployment

| Concern | Choice |
|---|---|
| Static hosting | Cloudflare Pages |
| API | Axum in Docker on Fly.io, scale-to-zero |
| Database | Neon Postgres, separate from compute |
| Migrations | Fly `release_command`, before traffic cutover |
| Source of truth | Postgres for chart text; a private repo holds seed input and backups |

See [ADR-0001](adr/0001-postgres-source-of-truth-for-charts.md) and
[ADR-0004](adr/0004-fly-compute-neon-postgres.md).

## Known deviations from this document

None currently.
