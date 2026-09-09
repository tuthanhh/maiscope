# ADR-0006 — Typed sqlx queries over JSON-assembly views

**Status:** accepted
**Date:** 2026-09-07

## Context

The catalog was originally assembled as JSON inside Postgres views (`v_song`,
`v_sheet`, `v_sheet_obj`), so response shape was defined in SQL and unchecked by
the compiler.

## Decision

Move assembly into Rust with `sqlx::query_as!`, making `types.rs` the
compiler-checked response-shape seam. Delivered by the shipped
`server-catalog-typed-queries` feature (8 tickets); views dropped in migration
`20260907080000_drop_json_views`.

## Consequences

62 compile-time macros now mean `cargo build` requires a reachable database
unless an offline query cache is committed — which is why `.sqlx/` and
`SQLX_OFFLINE` are prerequisites for containerising at all.
