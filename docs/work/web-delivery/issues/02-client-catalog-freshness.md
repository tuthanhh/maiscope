# 02 — Client catalog freshness: ETag now, IndexedDB blob after

**What to build:** Two stages, deliberately sequenced.

**v1.0 — plain conditional fetch.** `stores/data.ts:loadData` fetches
`/catalog` with the browser's own cache honouring the `ETag`/`Cache-Control` from
`server-restructure` issue 07. No client-side persistence. Simple, and correct.

**v1.1 — blob cache + manifest probe.** Persist the raw catalog JSON in
**IndexedDB**, call `GET /sync/manifest` at launch (already implemented
server-side), and refetch `/catalog` only when `revision` changed. **Paint from
cache immediately, swap when new data arrives** — which fits the existing model,
since `preprocessData` produces a frozen graph that is replaced wholesale anyway.

Not SQLite: the client already holds the whole catalog in memory and filters
locally (commit `cd30221` reverted to client-side filtering), so a relational
layer would only be translated back into the same JSON shape.

**Blocked by:** 01, `server-restructure/issues/07-etag-caching.md`

**Status:** done — stage 1 (v1.0) and stage 2 (v1.1, 2026-11-16); one
devtools confirmation owed, see Comments

*Stage 1 (v1.0)*
- [x] `loadData` uses plain `fetch`, no `isTauri()` branch (issue 01) — now via
      `utils/api.ts:fetchJson`, which also checks `response.ok`
- [x] Conditional requests verified server-side (see Comments)
- [ ] Confirmed once in browser devtools — see Comments for what to expect, since
      it is *not* simply "second load returns 304"
- [x] Loading/error states unchanged for users

*Stage 2 (v1.1)*
- [x] IndexedDB wrapper storing the catalog as a **raw string**, not a structured
      clone of the parsed graph — cloning a 4.7MB object graph is slow
      (`utils/catalogCache.ts`; `fetchText` in `utils/api.ts` so the body is
      never parsed and re-serialised)
- [x] Launch flow: read cache → paint → probe `/sync/manifest` → refetch on
      revision change → swap (`stores/data.ts:loadData`)
- [x] Cache miss / corrupt cache falls back to a normal fetch — an entry that
      fails `JSON.parse` is deleted first
- [x] Stored alongside the revision it came from
- [x] Note: Android WebView can evict IndexedDB under storage pressure. Acceptable
      — the cache is rebuildable — but the empty-cache path must be correct.
      Every wrapper call swallows IndexedDB failure into a miss, including
      a blocked `open` and a missing `indexedDB` global.


## Comments

**Stage 1 needed no new code.** Removing the Tauri transport (issue 01) left the
plain `fetch` this stage asks for, and `server-restructure` 07 had already shipped
the response headers. Verified against a local server holding the real 3MB catalog:

```
GET /api/v1/catalog
  200, etag: "bcd28b1d…", cache-control: public, max-age=3600, 3040454 bytes

GET /api/v1/catalog  (If-None-Match: "bcd28b1d…")
  304 Not Modified, 0 bytes
```

**The remaining checkbox is worded misleadingly, so here is what to actually
expect.** `max-age=3600` means the second load *within the hour* is served from
the browser's own cache with **no request at all** — better than a 304, but it
will not appear as one in devtools. A conditional request only happens once the
entry goes stale, or on a normal reload. A hard reload (Ctrl+Shift+R) bypasses the
cache entirely and returns a fresh 200. All three are correct; only the middle one
shows `304`.

**Stage 2 remains v1.1** and is untouched. Note that the plan's "paint from cache,
swap when new data arrives" still holds, but the `ETag` route already gets most of
the bytes saving — stage 2's real win is *first paint before the network*, not
transfer size.

**Stage 2 shipped 2026-11-16, verified by type-check and build only** — no
browser on the machine that wrote it. Three things to confirm once in
devtools, which also closes stage 1's remaining box:

1. Application → IndexedDB → `maiscope` → `catalog` holds one `current`
   entry with `revision`, `raw` (3MB string) and `storedAt`.
2. Second launch, Network panel: one `GET /sync/manifest` (200 or 304) and
   **no** `/catalog` request. That is the stage-1 "no request within the
   hour" expectation, now true for as long as the revision holds.
3. Airplane mode, reload: the catalog paints (before this change only the
   app shell did), with a console warning that the refresh failed.

**One decision made in passing.** The catalog is refetched with
`cache: "no-cache"`, so the browser revalidates instead of answering from
its own copy. Otherwise a manifest that says "changed" could be answered
from an HTTP cache up to `max-age=3600` behind it, and the stale body would
be stored under the new revision and never refetched. With IndexedDB in
front, `/catalog` is requested only on a miss or a change, so the HTTP
cache's hour of unconditional reuse no longer buys anything there; the
`ETag` still does.

**Selections survive the swap.** `preprocessData` freezes the graph and the
store replaces it wholesale, which would leave `currentSelectedSheets`
pointing into the old graph. They are re-resolved by `sheetExpr`; a sheet
that no longer exists drops out of the selection.
