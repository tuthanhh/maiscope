# 01 — Delete `src-tauri`, collapse to a single `fetch` path

**What to build:** Remove the Tauri shell entirely and unify how the frontend
talks to the server.

Today there are two transports for the same URL. `stores/data.ts:52` branches on
`isTauri()`: the native path goes `invoke("load_chart_data")` →
`src-tauri/src/data.rs` → `reqwest::get()`, which parses the full 4.7MB into a
`serde_json::Value` and re-serialises it across the IPC bridge. That proxy exists
only to dodge CORS, and it costs real things — `reqwest` there has no gzip
feature and sends no `If-None-Match`, so the desktop path cannot benefit from
`server-restructure` issue 07 at all.

With the target now web + PWA, the proxy has no reason to exist.

**Blocked by:** None (independent of server work)

**Status:** in-progress

- [x] `apps/host/src-tauri/` deleted, including `data.rs` and `search.rs`
- [x] `Cargo.toml` workspace members updated — `apps/host/src-tauri` removed
      (the stale member was breaking `cargo metadata` outright; fixed while
      closing `build-and-deploy` 02)
- [x] `isTauri()` branches removed from `stores/data.ts` and
      `composables/useSheetSearch.ts`; plain `fetch` everywhere
- [x] `@tauri-apps/api` removed from `apps/host/package.json`, along with
      `@tauri-apps/plugin-opener`, `@tauri-apps/cli` and the dead
      `"tauri": "tauri"` script. No `@tauri-apps/*` dependency remains
- [x] `tauri.conf.json`, `capabilities/`, Tauri icons removed
- [x] No Tauri references left in README, CLAUDE.md or `docs/guides/` — there were
      none by this point. `vite.config.ts` lost its `TAURI_DEV_HOST` binding, HMR
      block and `src-tauri` watch exclusion
- [x] `cargo build --workspace` green with the member gone; CI has been green on it since `56a9412`
- [x] Server CORS allowlist covers **local dev** — `http://localhost:1420` in both
      `apps/server/.env` and `fly.toml`. The Pages origin does not exist yet and is
      tracked as a checkbox on `web-delivery` 03


## Comments

**The Tauri path was only ever a CORS workaround.** Both branches invoked a Rust
command that fetched the *same* URL the browser path fetches — `load_chart_data`
for `/catalog`, `search_sheets` for `/sheets/search`. With the server's CORS
allowlist in place (`server-restructure` 06) the proxy has nothing left to do, so
collapsing it is a pure deletion with no behaviour change.

**`vite.config.ts` keeps `port: 1420` / `strictPort: true`** — no longer because
Tauri demands a fixed port, but because that origin is allowlisted in
`apps/server/.env` and in `fly.toml`. A port shuffle would silently break every
cross-origin request. The comment now says so.

Removing `host: host || false` does not widen exposure: with `TAURI_DEV_HOST`
unset it evaluated to `false`, and Vite's default is localhost either way.

**`about.vue` advertised the dropped architecture.** Its roadmap listed "Local
SQLite cache (tauri-plugin-sql)", now reworded to the actual plan (ETag, then
IndexedDB — `web-delivery` 02). Note item **C3 still promises "GitHub login +
contribution UI"**, which [ADR-0002](../../../adr/0002-chart-data-only-no-audio-hosting.md)
cut. Left alone deliberately: that is product copy, not a Tauri removal, and
whether v1 still advertises contributions as forthcoming is a decision, not a typo.

**Both calls now go through `utils/api.ts:fetchJson`, which checks
`response.ok`.** Previously `await (await fetch(url)).json()` parsed an error body
as if it were data, so the `/catalog` 500 production currently returns
(`prod-data-and-infra` 06) surfaced as a confusing failure inside `preprocessData`.
Pre-existing on the browser path rather than a regression from this ticket — but
collapsing the transports made it the *only* path, so it was worth closing here.

Non-2xx throws `ApiError`, carrying `status` so callers can branch later. The body
is read exactly once (it is a stream), and the parse is guarded with `.catch`
because a failure is not guaranteed to be JSON — a Fly 502 or a cold-start timeout
returns HTML or nothing, and an unguarded `json()` would replace the real failure
with a parse error. Verified against the live 500: the user-visible message becomes
`internal error` rather than a crash.

**Two follow-ups this exposes, both left open deliberately:**

- `internal error` is the server's intentionally opaque text, and `stores/data.ts`
  renders it verbatim. Wrapping it with context ("Couldn't load the song catalog")
  is a UI-copy decision, not part of removing Tauri.
- `useSheetSearch.runSearch` has only a `finally`, so a throw now propagates as an
  unhandled rejection and leaves stale results on screen. Previously it failed
  silently with garbage data. More honest, still not handled — whether search
  surfaces its own error state is a UX decision.
