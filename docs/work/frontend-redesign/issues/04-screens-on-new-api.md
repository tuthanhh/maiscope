# 04 — Screens against the new API

**What to build:** every existing screen reads `/api/...`.

**Blocked by:** [`api-rewrite`](../../api-rewrite/spec.md), 01, 02.

**Status:** todo

- [ ] `stores/data.ts:loadData`, `useSheetSearch`, `useEngine` and the song and
      sheet pages all call the new endpoints
- [ ] `preprocessData` reflects ticket 01's decision — kept with a stated reason,
      reduced, or deleted
- [ ] No request goes to a path containing a percent-encoded title
- [ ] The IndexedDB catalog cache and the `/sync` revision probe still work, or
      are deliberately changed — the delivery model was explicitly left alone by
      `api-rewrite`, so a change here is a scope increase worth naming
- [ ] The IndexedDB cache is versioned by payload shape (a new database or
      store name), so a v1-shaped Catalog cached before the cutover is never
      read as v2. The server also bumps the revision once at the v2 deploy
      (migration `20261117130000`); this is the client's half, and the one
      that does not depend on a migration having run
- [ ] After the manifest reports a new revision, the Catalog and reference
      tables are refetched with `cache: 'no-cache'` — both are
      `max-age=3600`, so a plain fetch can return the previous revision from
      the browser cache (contract §0)
- [ ] The filter panel's "Catalog not loaded" check reads the store's loading
      status: v2 does not send `updateTime`, which it used as the sentinel
