# Architecture

How the pieces fit together. Decisions and their rationale live in [`adr/`](../adr/);
this document describes the result.

## Tiers

```
apps/host/     Vue 3 + Vite web app — song browser and visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm32-unknown-unknown
```

One Cargo workspace (`apps/server`, `engine`) plus a pnpm-managed frontend
under `apps/host`. There is no shared-types crate: the server emits raw fields
the client reshapes, and a snapshot test guards the wire contract instead
([ADR-0018](../adr/0018-no-shared-types-crate.md)).

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

Two layers today, a third planned. Knowing which is which is the difference
between debugging staleness in one minute and three.

| Layer | Holds | Invalidated by |
|---|---|---|
| Service worker | App shell (precached); wasm (runtime, cache-first); sprites (stale-while-revalidate) | Content-hashed filenames; `index.html` is network-first; `/api/*` is never cached here |
| HTTP | `/catalog` and other wire responses | `ETag` / `If-None-Match`; `Cache-Control: max-age=3600` on `/catalog` |
| IndexedDB — **not built** | Catalog blob, for first paint before the network | `GET /sync/manifest` revision change; paint-then-swap |

The IndexedDB layer is
[`web-delivery` 02 stage 2](../work/web-delivery/issues/02-client-catalog-freshness.md),
deferred to v1.1. Nothing in `apps/host/` reads IndexedDB or calls
`/sync/manifest` yet; the catalog comes straight from `GET /catalog` through the
browser's HTTP cache (`stores/data.ts:loadData`).

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
[ADR-0002](../adr/0002-chart-data-only-no-audio-hosting.md).

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

See [ADR-0001](../adr/0001-postgres-source-of-truth-for-charts.md) and
[ADR-0004](../adr/0004-fly-compute-neon-postgres.md).

## Known deviations from this document

None currently. The IndexedDB cache layer is listed above as not built rather
than as a deviation: the design is decided, the code is scheduled for v1.1.
