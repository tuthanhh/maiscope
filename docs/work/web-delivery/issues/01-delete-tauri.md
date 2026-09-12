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
- [ ] `isTauri()` branches removed from `stores/data.ts:7,51` and
      `composables/useSheetSearch.ts:3,42`; plain `fetch` everywhere
- [ ] `@tauri-apps/api` removed from `apps/host/package.json` — **blocked on the
      branch removal above**, since both files still import `invoke, isTauri`
      from `@tauri-apps/api/core`. `@tauri-apps/plugin-opener`, `@tauri-apps/cli`
      and the dead `"tauri": "tauri"` script were already removed; they had no
      remaining references
- [ ] `tauri.conf.json`, `capabilities/`, Tauri icons removed
- [ ] `pnpm tauri dev` / `tauri build` references removed from README and CLAUDE.md
- [ ] `cargo build --workspace` still green with the member gone
- [ ] Server CORS allowlist (`server-restructure` issue 06) covers the Pages origin and local dev
