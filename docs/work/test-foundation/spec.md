# Spec — Test foundation

**Status:** planned
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Give the two tiers that have no safety net one: the simai parser, and the eight
server routes plus the response contract they promise.

## Problem

`engine/` has **zero** tests — the densest, most bug-prone, most user-visible code
in the repo is entirely unverified. `apps/host/` has none either. The server has
six `#[sqlx::test]` handler tests, fewer than CLAUDE.md claimed until
[`docs-restructure`](../docs-restructure/spec.md) corrected it.

Parser first, ahead of server tests, on four grounds:

- **Highest defect density.** simai is a gnarly text format — mid-chart BPM changes,
  slide syntax, touch notes, `/` and `*` groupings, `h[...]` holds.
- **Zero infrastructure.** Pure functions, native `cargo test -p engine`. No
  Postgres, no browser, no wasm. Milliseconds in CI.
- **Failures are silent.** A bad SQL change 500s loudly into the logs. A misparsed
  slide renders a *plausible but wrong* chart — nobody reports it, and if they do it
  cannot be reproduced without the exact chart text.
- **It is the phase-2 prerequisite.** Community charts will feed the parser syntax
  nobody has seen. Without a net, every contributed chart is a potential visualizer
  crash.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Engine: simai parser test suite](issues/01-engine-parser-tests.md) | Blocked by nothing — start any time |
| 02 | [Server: endpoint coverage + `/catalog` contract snapshot](issues/02-server-tests-and-contract-snapshot.md) | Blocked on `server-restructure` 04 and 07 |

## Decisions that constrain this work

- **A snapshot test, not the `shared` crate.** Generated types would not match what
  the frontend actually uses: the server deliberately emits raw fields only, while
  `preprocessData` adds `songNo`, `imageUrl`, `sheetExpr`, `notePercents` and
  `$canonicalSheet` client-side. In a read-only v1 the only real failure is the
  server dropping or renaming a field the client reads — and a snapshot catches
  exactly that. It also asserts the **absence** of derived fields, so the server
  cannot start emitting them.
- **A parser must never panic on malformed input.** A panic in wasm takes the whole
  canvas down; an error is recoverable.
- **Fixtures are chart text only** — never mp3, never bg images. The original 16
  `maidata.txt` were deleted with the audio on 2026-09-09 and have to be
  re-acquired into the private data repo.
- **Contract coverage lands here, not in `server-restructure`.** The ETag/304 and
  rate-limit-key tests written during those tickets get folded in, so one feature
  owns the regression surface.

## Out of scope

Frontend tests (`apps/host/`) — no test runner is configured and v1.0 does not add
one. Browser or end-to-end testing of the visualizer; the wasm boundary is exercised
by [`mobile-webview-spike`](../mobile-webview-spike/) instead.

## Done when

`cargo test -p engine` covers every note kind, timing case and a real chart corpus
without panicking, all eight routes have handler coverage including
`/sync/delta`'s `409` path and `/sheets/search` pagination, and a `/catalog`
snapshot fails the build when a field is dropped, renamed, or wrongly added.
