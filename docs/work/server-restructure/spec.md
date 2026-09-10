# Spec — Server restructure

**Status:** active
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Make `apps/server` deployable: configuration, errors, state, module boundaries,
observability, and the HTTP-layer work that decides whether the free tier holds.
Restructured **in place**, not rewritten — see
[ADR-0005](../../adr/0005-restructure-server-in-place.md).

## Problem

`main.rs` is 614 lines holding bootstrap, routing, domain enums, five query-param
DTOs, error helpers, eight handlers and the test module. `queries.rs` is 866 lines
of every query for every resource. Beyond size, four things block deployment
outright:

- `std::env::var("DATABASE_URL").unwrap()` and a hardcoded `0.0.0.0:3000` bind —
  Fly injects `PORT`, so the server cannot start there.
- No compression layer. `GET /catalog` ships **4.7MB uncompressed** per call and
  Fly's proxy does not gzip for you.
- No `ETag` wiring, though the contract specifies it and `catalog_hash` already
  exists. A repeat visitor pays 4.7MB instead of a 200-byte `304`.
- No `tracing` at all — one `println!`. A production failure produces nothing to
  look at.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [`config.rs`, environment-driven, no `unwrap`](issues/01-config-module.md) | Unblocks 03, 05, 06 |
| 02 | [One `AppError` implementing `IntoResponse`](issues/02-apperror.md) | Error contract enforced by the compiler, not convention |
| 03 | [`AppState` instead of a bare `Pool<Postgres>`](issues/03-appstate.md) | |
| 04 | [Split `main.rs` and `queries.rs` per resource](issues/04-split-modules.md) | Move code, do not rewrite it |
| 05 | [`tracing` + request spans](issues/05-tracing.md) | JSON to stdout; Fly captures stdout |
| 06 | [Compression, CORS allowlist, configurable bind](issues/06-http-layer.md) | 4.7MB → ~500KB |
| 07 | [`ETag` / `Cache-Control` / `304`](issues/07-etag-caching.md) | Highest-leverage free-tier protection |
| 08 | [Per-IP rate limiting keyed on `Fly-Client-IP`](issues/08-rate-limiting.md) | Blast-radius cap, not the main defence |
| 09 | [Extract "apply a chart revision"; `seed_songs` fails loudly](issues/09-chart-revision-service-fn.md) | The one piece of the write path that carries to phase 2 |

## Decisions that constrain this work

- **Restructure in place** ([ADR-0005](../../adr/0005-restructure-server-in-place.md)).
  Ticket 04 moves code; handler bodies and SQL stay byte-identical wherever
  possible, so the six existing `#[sqlx::test]` tests remain a real safety net.
- **Additive-only response shapes** (umbrella spec, rule 2). An installed PWA can
  run an arbitrarily old frontend against the current API. Route paths and the
  `/api/v1` nest do not move.
- **Raw fields only.** The server never emits `sheetExpr`, `imageUrl` or
  `notePercents`; `preprocessData` derives them client-side.
- **CORS here is an egress control, not a security control.** `curl` ignores it.
  The real cap is ticket 08.

## Out of scope

Any write endpoint. v1.0 has no contributions and no auth — a token-guarded admin
endpoint would be throwaway. Ticket 09 extracts the *service function* the phase-2
approve handler will call, and nothing more.

## Done when

`cargo test --workspace` is green throughout, `main.rs` is under ~80 lines,
`/catalog` responds `Content-Encoding: br` and answers `If-None-Match` with a
`304`, two different `Fly-Client-IP` values get independent rate-limit buckets, and
no environment access anywhere `unwrap`s.
