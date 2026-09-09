# ADR-0007 — Client-side filtering over server-side search

**Status:** accepted
**Date:** 2026-09-07

## Context

Server-side sheet search was designed and built across two features
(`server-side-sheet-search`, `frontend-server-side-search`), then reverted by
`cd30221` — "browse page back to client-side filtering, catalog is small
enough".

## Decision

The client holds the full catalog and filters locally. `GET /sheets/search` is
retained server-side but is not the browse path.

## Consequences

This is load-bearing for caching decisions. Because the whole catalog is in
memory anyway, a client-side SQLite cache would buy no query benefit and would
only translate rows back into the JSON shape `preprocessData` already wants.
That is why the offline cache is an IndexedDB blob plus a `/sync/manifest`
revision probe, not a relational store.
