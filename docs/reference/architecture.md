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
crates together with `--workspace` would undo that.

There is no shared-types crate: the server emits raw fields the client
reshapes, and the Catalog's mapping and forbidden-keys tests
(`apps/server/src/routes/catalog.rs`) guard the wire contract instead
([ADR-0018](../adr/0018-no-shared-types-crate.md)).

## Data flow

```
apps/host (browser)                    apps/server (Fly.io)
   │  GET /api/catalog          ──────▶  song/sheet metadata (Neon Postgres)
   │  GET /api/songs/{id}/sheets/{type}/{code}/chart ─▶ raw simai chart text
   ▼
visualizer page
   │  load_chart(text) via wasm-bindgen
   ▼
engine (Bevy, wasm)
   parses simai (via crates/simai) → spawns notes → renders on a wall-clock-driven chart clock
```

## Cross-tier identity

A song is its minted UUID (`songs.id`); a sheet is that id plus its type and
difficulty code — `/api/songs/{id}/sheets/{type}/{code}` on the server,
`sheet.key = ${id}/${type}/${code}` in the client (`utils/data.ts:sheetKey`)
([ADR-0026](../adr/0026-v2-api-carries-codes-and-read-fields-only.md)).
`sheet_expr` survives as the sync's upstream match key and as the join the
read queries use to hide tombstoned sheets (`queries/official.rs`); no route
takes or sends it.

## Raw versus derived fields

The server returns **raw fields a screen reads, nothing else**. The client
derives `imageUrl` and builds the explicit `sheet.song` reference and
`sheet.key` — `utils/data.ts:preprocessData` for the Catalog, `attachSong`
for a single song or sheet — then freezes the Catalog graph, which is
replaced wholesale on a refresh. Names and colours come from the reference
tables (`GET /api/reference`, fetched at boot), never per row. Keep new
endpoints on this split.

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
swap the graph wholesale, re-resolving bookmarks by sheet key (a key no
Catalog has resolved yet stays persisted, in `localStorage` `mv:bookmarks`).
A probe or refetch that fails after a cache hit is logged and the cached
catalog stays; on a miss Browse shows its error state with Retry. The store
is `utils/catalogCache.ts`: database `maiscope-catalog-v2` (the v1 `maiscope`
database is deleted on launch), object store `catalog`, one key `current`,
every failure a cache miss.

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
