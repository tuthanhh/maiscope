# Spec — Web delivery

**Status:** planned
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Ship the frontend as one web build: delete the Tauri shell, host on Cloudflare
Pages, make it installable as a PWA, and gate the visualizer off mobile until the
spike says otherwise. Web + PWA with Tauri dropped is
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
| 05 | [Gate the visualizer off mobile](issues/05-gate-visualizer-off-mobile.md) | Single flag, easy to flip for v1.1 |

## Decisions that constrain this work

- **Cloudflare Pages over serving the SPA from Axum.** Cloudflare's egress is
  unmetered and Fly's is billed; with a ~20MB wasm artifact that is the difference
  between free and a surprise invoice.
- **A custom domain is deferred, and the deferral has a trap.** A PWA's install
  identity is its origin. Anyone who installs from `*.pages.dev` before the domain
  move keeps a stale app pointed at the old origin. **Do not promote installs until
  the custom domain lands.**
- **`index.html` must be network-first.** A service worker is the one artifact you
  can ship that cannot be taken back over the network. A cache-first rule on
  `index.html` pins installed users to a broken build permanently.
- **Do not precache the wasm.** Precaching makes every visitor to the browse page
  download ~20MB before asking for the visualizer. Runtime cache-first on the
  content-hashed `.wasm` URL instead.
- **Silent auto-update, not prompt-to-update.** Nothing in a read-only app can be
  lost by reloading, so an update-prompt component is unearned complexity.
- **Not SQLite on the client.** The frontend already holds the whole catalog in
  memory and filters locally ([ADR-0007](../../adr/0007-client-side-filtering.md)),
  so a relational layer would only be translated back into the same JSON shape.
- **Feature detection, not user-agent sniffing**, for the mobile gate — and the
  gated device must never fire the dynamic import in `composables/useEngine.ts`, or
  it pays ~20MB for a page it cannot use.

## Cache layers this feature touches

Three layers, three lifetimes — the reason "why is my data stale" has three
possible answers. Service worker holds the app shell and the wasm; IndexedDB holds
the catalog blob (v1.1); HTTP holds wire responses via `ETag`. API responses are
**not** cached by the service worker — that is HTTP's job and IndexedDB's.

## Out of scope

Stage 2 of ticket 02 (IndexedDB blob cache) is v1.1, listed there to keep the
sequencing visible. The mobile visualizer itself is
[`mobile-webview-spike`](../mobile-webview-spike/) and v1.1. Any Tauri target,
permanently, unless the spike reopens ADR-0003.

## Done when

`src-tauri/` is gone and `cargo build --workspace` is still green without it, a
second catalog load returns `304` in devtools, the app installs on desktop Chrome
and Android Chrome, a broken deploy is provably recoverable on installed clients,
and a mobile visitor sees an honest "desktop only for now" without downloading the
wasm.
