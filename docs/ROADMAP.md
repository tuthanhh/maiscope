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

### v2.0 — community platform

maiscope stops being a mirror people read and becomes a place people put charts
into. GitHub authentication, a second library beside the official catalog
(anyone uploads a `maidata.txt`, anyone plays it), an API authored from scratch,
and a frontend rebuilt around all of it.

Published on upload with a report queue, not a moderation queue —
[ADR-0013](adr/0013-community-charts-beside-the-catalog.md), which supersedes the
contribution-queue design. Still no contributor audio
([ADR-0002](adr/0002-chart-data-only-no-audio-hosting.md)).

Rules, risks and build order: [`production-v2`](work/production-v2/spec.md).
Unscheduled.

## Features

| Feature | Status | Milestone |
|---|---|---|
| [production-v1](work/production-v1/spec.md) — umbrella | active | v1.0 |
| ├ [repo-hygiene](work/repo-hygiene/spec.md) | shipped | v1.0 |
| ├ [server-restructure](work/server-restructure/spec.md) | shipped | v1.0 |
| ├ [build-and-deploy](work/build-and-deploy/spec.md) | shipped | v1.0 |
| ├ [prod-data-and-infra](work/prod-data-and-infra/spec.md) | active | v1.0 |
| ├ [catalog-sync](work/catalog-sync/spec.md) | shipped | v1.0 |
| ├ [web-delivery](work/web-delivery/spec.md) | active | v1.0 |
| ├ [test-foundation](work/test-foundation/spec.md) | active | v1.0 |
| └ [api-pagination-header](work/api-pagination-header/spec.md) | shipped | v1.0 |
| [production-v2](work/production-v2/spec.md) — umbrella | planned | v2.0 |
| ├ [frontend-redesign](work/frontend-redesign/spec.md) | planned | v2.0 |
| ├ [catalog-difficulties](work/catalog-difficulties/spec.md) | planned | v2.0 |
| ├ [song-public-id](work/song-public-id/spec.md) | planned | v2.0 |
| ├ [parser-crate-extraction](work/parser-crate-extraction/spec.md) | planned | v2.0 |
| ├ [server-auth-github-oauth](work/server-auth-github-oauth/) | planned | v2.0 |
| ├ [api-rewrite](work/api-rewrite/spec.md) | planned | v2.0 |
| └ [community-charts](work/community-charts/spec.md) | planned | v2.0 |
| [server-contributions](work/server-contributions/spec.md) | superseded | — |
| [parser-defects](work/parser-defects/spec.md) | active | v1.0 |
| [mobile-webview-spike](work/mobile-webview-spike/) | planned | v1.1 |
| [docs-restructure](work/docs-restructure/spec.md) | shipped | — |
| [server-sync-tier](work/server-sync-tier/) | shipped | — |
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
