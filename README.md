# maiscope

A song browser and real-time chart visualizer for **maimai** (SEGA's arcade
rhythm game), built as a Rust/TypeScript monorepo: a Vue web app, a Rust backend
serving a community chart database, and a Bevy engine compiled to WebAssembly for
in-app chart playback.

## What's here

```
apps/host/     Vue 3 + Vite web app (PWA-installable) — song browser, visualizer page
apps/server/   Rust/Axum + Postgres — chart catalog and per-sheet chart API
engine/        Bevy ECS chart renderer, compiled to wasm and imported by apps/host
shared/        Rust crate for cross-workspace types (scaffolded, not yet wired in)
```

For how these fit together, see [docs/architecture.md](docs/architecture.md).

## Getting started

See [docs/guides/local-setup.md](docs/guides/local-setup.md).

## Features

- Song gallery with search and filtering (level, category, version, BPM, region)
- Per-sheet detail view: difficulty, internal level, note designer, note counts
- Chart visualizer: taps, holds, touch notes, slides with star and path traces
- Light/dark mode; UI in EN, JA, KO, ZH-Hans, ZH-Hant, VI, ES, ID, RU

## Documentation

| | |
|---|---|
| [Roadmap](docs/ROADMAP.md) | What is done, what is next |
| [Architecture](docs/architecture.md) | How the tiers fit together |
| [Decisions](docs/adr/) | Why it is built this way |
| [API contract](docs/reference/api-contract.md) | The HTTP surface |
| [Database schema](docs/reference/schema.md) | Postgres tables |
| [simai notation](docs/reference/simai-notation.md) | The chart text format |

## Acknowledgments

- **[zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)** — the web
  app this frontend is ported from. Data model, filtering logic and UI design
  credit belong to the upstream project.
- [mai-notes.com](https://mai-notes.com/) and [majdata.net](https://majdata.net/) —
  simai format and note-timing references
- [maimai でらっくす 公式サイト｜セガ](https://maimai.sega.jp/) — official song information

## License

MIT — see [LICENSE](./LICENSE). Third-party licenses (including the upstream
`zetaraku/arcade-songs` MIT notice this frontend is ported from) are in
[LICENSE-THIRD-PARTY](./LICENSE-THIRD-PARTY).
