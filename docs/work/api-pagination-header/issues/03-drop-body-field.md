# 03 — Drop `total` from the response body

**What to build:** the contract step. `SheetSearchResponse` loses its `total`
field; the header is the only source.

**Blocked by:** 02 — and specifically on 02 being **deployed**. Removing the
field while an older bundle is live breaks its pagination.

**Status:** dropped

- [ ] `types::SheetSearchResponse` is `{ sheets }`
- [ ] The handler still computes the total — it is the header's value now, not
      dead code
- [ ] The step-01 test asserting header-equals-body is rewritten to assert the
      header against the query's real match count
- [ ] `api-contract.md` §1.2 response example drops `total`; §6's Pagination
      bullet is rewritten to the header convention and points at
      [ADR-0015](../../../adr/0015-pagination-total-as-a-response-header.md)
- [ ] `community-charts` tickets 03 and 06 already say header — confirm, do not
      re-edit

## Dropped

v1 is frozen and scheduled for deletion under [`api-rewrite`](../../api-rewrite/spec.md). Removing a field from a version that is not being
evolved is churn: the body field costs nothing while it sits there, its only
consumer is a frontend that is migrating away, and the removal would be a
breaking change to `/api/v1` — which §6's versioning rule forbids and which this
ticket would have needed an exception for.

v2 is authored with the header only, natively. No migration, because it has no
legacy consumer to carry.

Tickets 01 and 02 already paid for themselves: 01 established that
`Access-Control-Expose-Headers` is required, with a test that fails without it,
so v2 inherits that as knowledge rather than rediscovering it in production; 02's
frontend change is forward-compatible.
