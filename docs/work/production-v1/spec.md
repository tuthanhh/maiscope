# Spec — Production v1.0

Decision record from the 2026-09-09 grilling session. Every choice below was
made explicitly; the rationale is kept because the *reasons* constrain future
work more than the choices do.

## Product shape

Milestones are defined in [`docs/ROADMAP.md`](../../ROADMAP.md). This is the
**umbrella spec for v1.0**: it holds the rules and risks that span every child
feature listed at the bottom, and owns no tickets itself.

**Explicitly out of v1**: audio hosting, auth, contributions, desktop app,
local SQLite cache, Tauri (any target).

## Architecture

| Layer | Decision |
|---|---|
| Frontend | One Vue web build, PWA-installable. `src-tauri/` deleted. |
| Static hosting | Cloudflare Pages. `*.pages.dev` first; custom domain later. |
| API | Axum on Fly.io, Docker image, scale-to-zero. |
| Database | Neon Postgres, free tier, **separate from compute** so a redeploy can never touch data. |
| Source of truth | Postgres for chart text + revisions. A private repo is the seed *input* and *backup*, never the authority. |
| Upstream catalog | Vendored `data.json` snapshot in-repo; a scheduled job opens a PR to refresh it. |

### Decisions behind this scope

| Decision | ADR |
|---|---|
| Postgres is the source of truth for chart text | [ADR-0001](../../adr/0001-postgres-source-of-truth-for-charts.md) |
| Chart data only, no audio hosting | [ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md) |
| Web + PWA, Tauri dropped | [ADR-0003](../../adr/0003-web-pwa-drop-tauri.md) |
| Fly compute + Neon Postgres | [ADR-0004](../../adr/0004-fly-compute-neon-postgres.md) |
| Restructure the server in place | [ADR-0005](../../adr/0005-restructure-server-in-place.md) |

## Cross-cutting rules

1. **Expand-and-contract migrations.** Additive migrations may deploy
   automatically. Destructive ones (`DROP COLUMN`, `DROP TABLE`) are a separate,
   deliberate, manual step. Postgres holds the only copy of chart text.
2. **Additive-only response shapes.** An installed PWA can run an arbitrarily
   old frontend against the current API. Never remove or rename a response field.
3. **`ingest` truncates.** The catalog-reload job and the chart-seed job must
   stay separate so the destructive one is never fired by reflex.
4. **No PII in v1.** No accounts, no analytics identifiers. Keeps the project out
   of needing a privacy policy at all — preserve this when adding client error
   reporting.

## Three cache layers, three lifetimes

Worth writing down because "why is my data stale" otherwise has three possible
answers and the wrong one gets checked first.

| Layer | Holds | Invalidated by |
|---|---|---|
| Service worker | App shell (precached), wasm (runtime, cache-first by hashed URL) | Content-hashed filenames; `index.html` is network-first |
| IndexedDB | Catalog blob (v1.1+; v1.0 is plain ETag fetch) | `GET /sync/manifest` revision change, paint-then-swap |
| HTTP | Wire responses | `ETag` / `If-None-Match` / `Cache-Control` |

## Verify before relying on (facts I do not trust)

- Neon's **actual** free-tier restore window. Decides whether "roll back the
  data" is true.
- Whether Neon's free tier allows a branch per PR (would let CI run
  `#[sqlx::test]` against real ephemeral Postgres).
- Upstream `zetaraku/arcade-songs` license terms — the README credits it in
  prose, which is not the same as retaining a copyright notice.
- Release wasm size, raw and brotli'd.

## Known-unresolved risks

- **Bevy on mobile WebView is unproven.** A bad spike result reopens the Tauri
  decision; an APK bundling the wasm would become the only path to v1.1.
- **PWA install identity is its origin.** Moving `*.pages.dev` → custom domain
  orphans installed apps. Do not promote installs until the domain lands.
- **`seed_songs` matches by NFC-normalized title.** Upstream renames silently
  drop charts (`server-restructure` issue 09 makes this loud;
  `prod-data-and-infra` issue 02 makes it reviewable).

## Child features

This directory is an **umbrella**: it holds the cross-cutting rules above and no
tickets of its own. The work is split across the feature directories below, each
with its own `spec.md`. Build them roughly in this order — the ordering is the
dependency graph, not a preference.

| Order | Feature | Why here |
|---|---|---|
| 1 | [`repo-hygiene`](../repo-hygiene/spec.md) | A live hazard — `songs/` is untracked but not ignored, and the upstream licence is unverified. Do it before the repo goes public. |
| 2 | [`server-restructure`](../server-restructure/spec.md) | Config, errors, state, modules, tracing, HTTP layer, caching, rate limiting. In place, tests green throughout ([ADR-0005](../../adr/0005-restructure-server-in-place.md)). |
| 3 | [`build-and-deploy`](../build-and-deploy/spec.md) | Hermetic build and deploy mechanics: `.sqlx/`, Dockerfile, `fly.toml`, CI, CD. |
| 4 | [`prod-data-and-infra`](../prod-data-and-infra/spec.md) | Neon provisioning, the vendored upstream snapshot and its refresh PR, seed workflows, backups. |
| 5 | [`web-delivery`](../web-delivery/spec.md) | De-Tauri, catalog freshness, Cloudflare Pages, PWA, the mobile visualizer gate. |
| 6 | [`test-foundation`](../test-foundation/spec.md) | Engine parser tests first — highest defect density, zero infrastructure — then server tests and the contract snapshot. |
| — | [`mobile-webview-spike`](../mobile-webview-spike/) | Gates **v1.1**, not v1.0. Run it early and in parallel; a bad result reopens [ADR-0003](../../adr/0003-web-pwa-drop-tauri.md). |

The split itself, and the rules it changed, are recorded in
[ADR-0009](../../adr/0009-umbrella-features-and-spec-timing.md). One ticket from the
original 29 is in no directory: `29 — docs sync` was dropped because the
[`docs-restructure`](../docs-restructure/spec.md) feature absorbed it and shipped.

The "verify before relying on" list above is owned by
[`prod-data-and-infra`](../prod-data-and-infra/spec.md) ticket 01 — treat those
facts as its acceptance criteria rather than as background reading.
