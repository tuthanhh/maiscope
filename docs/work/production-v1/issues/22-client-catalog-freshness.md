# 22 — Client catalog freshness: ETag now, IndexedDB blob after

**What to build:** Two stages, deliberately sequenced.

**v1.0 — plain conditional fetch.** `stores/data.ts:loadData` fetches
`/catalog` with the browser's own cache honouring the `ETag`/`Cache-Control` from
issue 08. No client-side persistence. Simple, and correct.

**v1.1 — blob cache + manifest probe.** Persist the raw catalog JSON in
**IndexedDB**, call `GET /sync/manifest` at launch (already implemented
server-side), and refetch `/catalog` only when `revision` changed. **Paint from
cache immediately, swap when new data arrives** — which fits the existing model,
since `preprocessData` produces a frozen graph that is replaced wholesale anyway.

Not SQLite: the client already holds the whole catalog in memory and filters
locally (commit `cd30221` reverted to client-side filtering), so a relational
layer would only be translated back into the same JSON shape.

**Blocked by:** 08, 21

**Status:** todo

*Stage 1 (v1.0)*
- [ ] `loadData` uses plain `fetch`, no `isTauri()` branch (issue 21)
- [ ] Verified in devtools: second load returns `304`, near-zero bytes
- [ ] Loading/error states unchanged for users

*Stage 2 (v1.1)*
- [ ] IndexedDB wrapper storing the catalog as a **raw string**, not a structured
      clone of the parsed graph — cloning a 4.7MB object graph is slow
- [ ] Launch flow: read cache → paint → probe `/sync/manifest` → refetch on
      revision change → swap
- [ ] Cache miss / corrupt cache falls back to a normal fetch
- [ ] Stored alongside the revision it came from
- [ ] Note: Android WebView can evict IndexedDB under storage pressure. Acceptable
      — the cache is rebuildable — but the empty-cache path must be correct.
