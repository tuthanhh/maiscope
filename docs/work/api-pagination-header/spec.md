# Spec — Pagination total as a response header

**Status:** shipped
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Move the paginated total from the response body to an `X-Total-Count` header.
Decided in [ADR-0015](../../adr/0015-pagination-total-as-a-response-header.md).

## Problem

`api-contract.md` §6 puts the total in the body, and gives a reason that is about
to stop being true:

> the only paginated endpoint (`GET /sheets/search`) returns an envelope
> already, so a header would be a second place to look

[`community-charts`](../community-charts/spec.md) adds two more paginated
endpoints. Once there are three, where the count lives should be a convention
rather than a per-endpoint judgement call.

Doing it now rather than later means `community-charts` is built against the
final convention, and `test-foundation` 02 tests the end state instead of
assertions that get rewritten.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Emit the header, keep the field](issues/01-emit-header.md) | Expand |
| 02 | [Frontend reads the header](issues/02-frontend-switch.md) | Blocked by 01 **being deployed** |
| 03 | [Drop the body field](issues/03-drop-body-field.md) | **dropped** — v1 is frozen, see [`api-v2`](../api-v2/spec.md) |

## Decisions that constrain this work

- **Expand-and-contract, three separate deploys.** The endpoint is shipped with
  a live consumer. Each ticket is independently deployable and there is no
  window where either tier is broken — but only if they go in order.
- **`expose_headers` is not optional.** Without it the browser hides the header
  from JS, with no error, cross-origin only. A same-origin test passes and
  production breaks.
- **Offset pagination stays.** Keyset was considered and deferred; see
  ADR-0015's consequences for the trigger to revisit.

## Deploy order

This is the part that cannot be automated away, and the reason the tickets are
split at all:

```
ticket 01  →  dispatch deploy.yml      (server: header + field)
ticket 02  →  dispatch deploy-web.yml  (frontend: reads header)
ticket 03  →  dispatch deploy.yml      (server: field removed)
```

Committing all three and deploying once would work, but deploying 03's server
before 02's frontend breaks pagination for anyone holding the old bundle.

## Out of scope

`community-charts`' own endpoints — they are `planned`, and ticket 03 there is
already written against this convention.

## Done when

`/sheets/search` returns `X-Total-Count` and the header is readable
cross-origin. Both hold.

The original third condition — `SheetSearchResponse` having no `total` field —
is dropped along with ticket 03: v1 is frozen, so it keeps both until it is
deleted wholesale. The convention itself carries forward into v2, where it is
native rather than migrated.
