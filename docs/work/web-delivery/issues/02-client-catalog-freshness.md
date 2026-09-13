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

**Status:** in-progress — stage 1 done (v1.0); stage 2 deferred to v1.1

*Stage 1 (v1.0)*
- [x] `loadData` uses plain `fetch`, no `isTauri()` branch (issue 01) — now via
      `utils/api.ts:fetchJson`, which also checks `response.ok`
- [x] Conditional requests verified server-side (see Comments)
- [ ] Confirmed once in browser devtools — see Comments for what to expect, since
      it is *not* simply "second load returns 304"
- [x] Loading/error states unchanged for users

*Stage 2 (v1.1)*
- [ ] IndexedDB wrapper storing the catalog as a **raw string**, not a structured
      clone of the parsed graph — cloning a 4.7MB object graph is slow
- [ ] Launch flow: read cache → paint → probe `/sync/manifest` → refetch on
      revision change → swap
- [ ] Cache miss / corrupt cache falls back to a normal fetch
- [ ] Stored alongside the revision it came from
- [ ] Note: Android WebView can evict IndexedDB under storage pressure. Acceptable
      — the cache is rebuildable — but the empty-cache path must be correct.


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
