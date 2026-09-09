# ADR-0003 — Web + PWA; Tauri dropped entirely

**Status:** accepted
**Date:** 2026-09-09

## Context

The frontend shipped as a Tauri desktop app whose Rust side proxied catalog
fetches to dodge CORS. The user's targets are Android and web, not desktop, and
called adopting Tauri a mistake. `tauri android init` had never been run —
`gen/` held only `schemas`.

## Decision

One web build, PWA-installable. `src-tauri/` deleted.

## Rejected alternatives

Tauri Android. Its only genuine advantage was bundling the ~20MB wasm into an
APK, which a service worker largely recovers. Against that: a second toolchain,
a second release pipeline, signing key custody, Play Store review, and
scoped-storage complexity for any future local song packs.

## Consequences

A single `fetch` path gains ETag, compression and IndexedDB caching at once —
the Rust proxy could not do any of them (it parsed 4.7MB into a
`serde_json::Value` and re-serialised it over IPC, with no gzip and no
`If-None-Match`). A CORS allowlist is needed until the custom domain lands. A
PWA's install identity is its origin, so moving from `*.pages.dev` later orphans
installed apps.

**This decision reopens if the mobile WebView spike
(`work/mobile-webview-spike/issues/01-bevy-mobile-webview-spike.md`) shows Bevy cannot hold
frame rate in an Android WebView.** In that case an APK bundling the wasm becomes
the only path to v1.1.
