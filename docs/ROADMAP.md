# Roadmap

The single index of what is done, what is next, and in what order. Every other
document links here rather than restating it.

## Milestones

### v1.0 — public web app, browse-only

Catalog browsing, search and sheet details, deployed publicly. The visualizer is
reachable on desktop browsers and gated off mobile.

Out of scope, deliberately: audio hosting, authentication, contributions, a
desktop app, and a local SQLite cache. See [ADR-0002](adr/0002-chart-data-only-no-audio-hosting.md)
and [ADR-0003](adr/0003-web-pwa-drop-tauri.md).

### v1.1 — mobile visualizer

The Bevy visualizer running in a mobile browser and installed PWA. Gated on the
spike in `work/mobile-webview-spike/issues/01-bevy-mobile-webview-spike.md`, whose
outcome can reopen [ADR-0003](adr/0003-web-pwa-drop-tauri.md).

### Phase 2 — community charts

GitHub authentication, the contribution and moderation queue, and
contributor-supplied audio. Unscheduled.

## Features

| Feature | Status | Milestone |
|---|---|---|
| [production-v1](work/production-v1/spec.md) — umbrella | active | v1.0 |
| ├ [repo-hygiene](work/repo-hygiene/spec.md) | planned | v1.0 |
| ├ [server-restructure](work/server-restructure/spec.md) | shipped | v1.0 |
| ├ [build-and-deploy](work/build-and-deploy/spec.md) | planned | v1.0 |
| ├ [prod-data-and-infra](work/prod-data-and-infra/spec.md) | planned | v1.0 |
| ├ [web-delivery](work/web-delivery/spec.md) | planned | v1.0 |
| └ [test-foundation](work/test-foundation/spec.md) | planned | v1.0 |
| [mobile-webview-spike](work/mobile-webview-spike/) | planned | v1.1 |
| [docs-restructure](work/docs-restructure/spec.md) | shipped | — |
| [server-sync-tier](work/server-sync-tier/) | shipped | — |
| [server-auth-github-oauth](work/server-auth-github-oauth/) | planned | phase 2 |
| [server-contributions](work/server-contributions/) | planned | phase 2 |
| [server-catalog-typed-queries](work/server-catalog-typed-queries/) | shipped | — |
| [server-side-sheet-search](work/server-side-sheet-search/) | shipped | — |
| [frontend-server-side-search](work/frontend-server-side-search/) | shipped | — |
| [chart-playback-encapsulation](work/chart-playback-encapsulation/) | shipped | — |

## Conventions

- Feature status: `planned` → `active` → `shipped` → `superseded`
- Ticket status: `todo` → `in-progress` → `done` → `dropped`
- A feature needs a `spec.md` before it goes `active`; `planned` features may be
  tickets only. An umbrella feature owns no tickets — it holds the rules its
  children share and indexes them.
- When a feature ships, its decisions move to [`adr/`](adr/), its behaviour
  changes land in [`reference/`](reference/), and this table collapses it to one
  line. Tickets stay in git but stop being roadmap surface.
