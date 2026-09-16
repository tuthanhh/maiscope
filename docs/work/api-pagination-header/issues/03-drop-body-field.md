# 03 — Drop `total` from the response body

**What to build:** the contract step. `SheetSearchResponse` loses its `total`
field; the header is the only source.

**Blocked by:** 02 — and specifically on 02 being **deployed**. Removing the
field while an older bundle is live breaks its pagination.

**Status:** todo

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
