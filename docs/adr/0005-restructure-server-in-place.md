# ADR-0005 — Restructure the server in place rather than rewrite

**Status:** accepted
**Date:** 2026-09-09

## Context

The user judged `apps/server` flawed and poorly structured, and proposed
rebuilding it. Inspection confirmed real problems: `main.rs` is 614 lines mixing
bootstrap, routing, domain enums, five query-param DTOs, error helpers, eight
handlers and tests; errors are ad-hoc `(StatusCode, Json<Value>)` tuples with no
`IntoResponse` type; state is a bare `Pool<Postgres>`; configuration is
`env::var(...).unwrap()`; `queries.rs` is 866 flat lines.

## Decision

Restructure in place, keeping tests green throughout.

## Rejected alternatives

Rewrite. Everything on that list is a move-code-around problem. What is *not*
on it — the schema, the SQL, the HTTP contract, the `sheet_expr` cross-tier key,
the sync-revision design — is sound and expensive to rebuild. A rewrite would
discard six passing `#[sqlx::test]` handler tests for the duration and require
retyping 62 `query_as!` macros by hand, which is exactly where a dropped
`COALESCE` or a lost `ORDER BY` enters.

## Consequences

Identical end state, reached incrementally. The existing tests are the safety
net, so they must keep passing at every step.
