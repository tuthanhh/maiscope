# 02 — Frontend reads the header

**What to build:** `useSheetSearch.ts` takes the total from `X-Total-Count`
instead of the response body.

Today (`useSheetSearch.ts:48`):

```ts
total.value = data.total;
```

`BrowsePagination.vue` consumes that value for its range display and page count;
nothing else changes.

**Blocked by:** 01 — and specifically on 01 being **deployed**, not merged. A
frontend reading a header the live server does not send gets `null`.

**Status:** todo

- [ ] Reads `response.headers.get("X-Total-Count")`, parsed to a number
- [ ] A missing or unparseable header falls back rather than rendering `NaN`
      pages — during the migration this should not happen, but the failure mode
      if `expose_headers` is ever dropped is silent, so it should degrade
      visibly rather than corrupt the page count
- [ ] `pnpm build` clean (`vue-tsc --noEmit` + `vite build`)
- [ ] Verified against a running server, not only by type-check — the CORS
      behaviour this depends on does not exist at compile time
