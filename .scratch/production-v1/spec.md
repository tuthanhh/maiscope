# Spec — Production v1.0

Decision record from the 2026-09-09 grilling session. Every choice below was
made explicitly; the rationale is kept because the *reasons* constrain future
work more than the choices do.

## Product shape

| Milestone | Contents |
|---|---|
| **v1.0** | Public web app, **browse-only**. Catalog, search, sheet details. Visualizer reachable on desktop browsers, gated off mobile. |
| **v1.1** | Mobile visualizer, gated on the Bevy/WebView spike (issue 28). |
| **Phase 2** (unscheduled) | Community charts, GitHub auth, contribution + moderation flow, contributor-supplied audio. |

**Explicitly out of v1**: audio hosting, auth, contributions, desktop app,
local SQLite cache, Tauri (any target).

### Why browse-only

The catalog browser is the entire upstream arcade-songs product and is useful
alone. The visualizer is the differentiator but Bevy has never been run in a
mobile WebView — that is the single unproven assumption in the plan, so it does
not gate the first release.

## Architecture

| Layer | Decision |
|---|---|
| Frontend | One Vue web build, PWA-installable. `src-tauri/` deleted. |
| Static hosting | Cloudflare Pages. `*.pages.dev` first; custom domain later. |
| API | Axum on Fly.io, Docker image, scale-to-zero. |
| Database | Neon Postgres, free tier, **separate from compute** so a redeploy can never touch data. |
| Source of truth | Postgres for chart text + revisions. A private repo is the seed *input* and *backup*, never the authority. |
| Upstream catalog | Vendored `data.json` snapshot in-repo; a scheduled job opens a PR to refresh it. |

### Rejected, and why

- **Static/bundled catalog with GitHub-PR contributions** — rejected because
  phase 2 writes go through `charts`/`chart_revisions` with a moderation queue;
  git cannot be the write path.
- **Tauri (desktop or Android)** — its only remaining advantage was bundling the
  wasm in an APK and native FS access for local song packs. Neither is a v1
  requirement, and a service worker recovers most of the first.
- **Serving the SPA from Axum** — a ~20MB wasm artifact on metered Fly bandwidth
  with no CDN.
- **Rewriting the server** — the schema, SQL, contract, `sheet_expr` identity key
  and sync-revision design are sound. Every real flaw is a move-code-around
  problem. Restructure in place, tests green throughout.

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
  drop charts (issue 10 makes this loud; issue 17 makes it reviewable).

## Ticket order

Issues `01`–`29` in `issues/`. Rough phases:

1. **01** hygiene — do first, it is a live hazard.
2. **02–10** server restructure in place, tests green throughout.
3. **11–13** hermetic build + deploy mechanics.
4. **14–20** infrastructure, CI/CD, data pipeline, backups.
5. **21–25** frontend: de-Tauri, hosting, PWA, mobile gate.
6. **26–27** tests (parser first — highest defect density, zero infra).
7. **28** spike that gates v1.1.
8. **29** docs sync.
