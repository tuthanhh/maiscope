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
