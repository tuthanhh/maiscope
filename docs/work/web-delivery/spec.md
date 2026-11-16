# Spec — Web delivery

**Status:** shipped — 2026-11-17
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Ship the frontend as one web build: delete the Tauri shell, host on Cloudflare
Pages, and make it installable as a PWA. The visualizer ships ungated on mobile
([ADR-0016](../../adr/0016-visualizer-ungated-on-mobile.md)). Web + PWA with Tauri dropped is
[ADR-0003](../../adr/0003-web-pwa-drop-tauri.md).

## Problem

There are two transports for the same URL. `stores/data.ts` branches on
`isTauri()`: the native path goes `invoke("load_chart_data")` → `src-tauri/src/data.rs`
→ `reqwest::get()`, parsing the full 4.7MB into a `serde_json::Value` and
re-serialising it across the IPC bridge. That proxy exists only to dodge CORS, and
`reqwest` there has no gzip feature and sends no `If-None-Match` — so the desktop
path cannot benefit from the caching work at all. With the target now web + PWA,
the proxy has no reason to exist.

The visualizer is the other half. Bevy on a mobile WebView is unproven, and
shipping a white screen or an OOM-killed tab as the headline feature is worse than
shipping the catalog browser alone — which is the entire upstream arcade-songs
product and is useful by itself.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Delete `src-tauri`, collapse to a single `fetch` path](issues/01-delete-tauri.md) | Independent of server work |
| 02 | [Client catalog freshness: ETag now, IndexedDB blob after](issues/02-client-catalog-freshness.md) | Stage 1 is v1.0; stage 2 is v1.1 |
| 03 | [Frontend hosting on Cloudflare Pages](issues/03-cloudflare-pages.md) | |
| 04 | [PWA: manifest + service worker](issues/04-pwa-service-worker.md) | |
| 05 | [Gate the visualizer off mobile](issues/05-gate-visualizer-off-mobile.md) | **Dropped.** It runs on mobile; see [ADR-0016](../../adr/0016-visualizer-ungated-on-mobile.md) |
| 06 | [Custom domain, with `/api/*` proxied to Fly](issues/06-custom-domain.md) | Dropped 2026-11-17 — `pages.dev` is the permanent origin |

## Decisions that constrain this work

- **Cloudflare Pages over serving the SPA from Axum.** Cloudflare's egress is
  unmetered and Fly's is billed; with a ~20MB wasm artifact that is the difference
  between free and a surprise invoice.
- **No custom domain.** `maiscope.pages.dev` is the permanent origin
  (ticket 06, dropped 2026-11-17). A PWA's install identity is its origin, so
  installs can be promoted; CORS and the Fly cold start stay as they are.

## Cache layers this feature touches

Three layers, three lifetimes — the reason "why is my data stale" has three
possible answers. Service worker holds the app shell and the wasm; IndexedDB holds
the catalog blob (v1.1); HTTP holds wire responses via `ETag`. API responses are
**not** cached by the service worker — that is HTTP's job and IndexedDB's.

## Out of scope

Stage 2 of ticket 02 (IndexedDB blob cache) is v1.1, listed there to keep the
sequencing visible. Mobile visualizer polish is
[`mobile-webview-spike`](../mobile-webview-spike/) and v1.1. Any Tauri target,
permanently, unless the spike reopens ADR-0003.

## Done when

`src-tauri/` is gone and `cargo build --workspace` is still green without it, a
second catalog load returns `304` in devtools, the app installs on desktop Chrome
and Android Chrome, and a broken deploy is provably recoverable on installed clients.
