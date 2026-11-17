# Architecture

How the pieces fit together. Decisions and their rationale live in [`adr/`](../adr/);
this document describes the result.

## Tiers

```
apps/host/     Vue 3 + Vite web app — song browser and visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm32-unknown-unknown
crates/simai/  simai chart and maidata.txt parser; no Bevy, no I/O
```

One Cargo workspace (`apps/server`, `crates/simai`, `engine`) plus a
pnpm-managed frontend under `apps/host`. `simai` is shared by the engine,
which parses chart text in the browser, and the server, which reads
`maidata.txt` when seeding and will validate uploads. Only the engine turns
on its `bevy` feature, so the server never links Bevy
([ADR-0014](../adr/0014-simai-parser-as-its-own-crate.md)); building the
crates together with `--workspace` would undo that. There is no shared-types crate: the server emits raw fields
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
   parses simai (via crates/simai) → spawns notes → renders on a wall-clock-driven chart clock
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

Three layers. Knowing which is which is the difference between debugging
staleness in one minute and three.

| Layer | Holds | Invalidated by |
|---|---|---|
| Service worker | App shell (precached); wasm (runtime, cache-first); sprites (stale-while-revalidate) | Content-hashed filenames; `index.html` is network-first; `/api/*` is never cached here |
| HTTP | `/sync/manifest` and other wire responses; `/catalog` only as a validator | `ETag` / `If-None-Match`; the catalog is fetched with `cache: "no-cache"`, so `max-age=3600` never answers it from the browser cache |
| IndexedDB | The raw `/catalog` body, under the `/sync/manifest` revision it was fetched for | A launch whose manifest `revision` differs from the stored one; paint-then-swap |

The launch flow (`stores/data.ts:loadData`): read IndexedDB → paint if hit →
`GET /sync/manifest` → refetch `/catalog` only when the revision changed →
swap the graph wholesale, re-resolving selected sheets by `sheetExpr`. A
probe or refetch that fails after a cache hit is logged and the cached
catalog stays; on a miss it is the user-visible error it always was. The
store is `utils/catalogCache.ts`: database `maiscope`, object store
`catalog`, one key `current`, every failure a cache miss.

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

Guide sounds and the metronome fire from the chart clock
(`systems/audio.rs`: `GuideScheduler`, `tick_metronome`), not from sprite
timers, and run ahead by `AudioLead` — a wall-clock lead the host derives
from `AudioContext.baseLatency + outputLatency` plus cpal's buffers and
pushes through `set_audio_lead_ms`
([ADR-0020](../adr/0020-guide-sounds-from-the-clock-with-a-derived-lead.md)).
The main camera renders without MSAA (`plugins/camera.rs`); multisampling was
the whole of the phone's frame cost. The engine also publishes a one-second
frame-pacing snapshot (`systems/frame_stats.rs`, read by `frame_stats()`),
which the visualizer's `?debug` overlay shows together with wasm memory.

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

None currently.
