# ADR-0001 — Postgres is the source of truth for chart text

**Status:** accepted
**Date:** 2026-09-09

## Context

Chart text existed only in an untracked local folder (16 songs, 196KB; the full
corpus is roughly 1500 songs, about 18MB of text). The alternative was a git
repository as the authority, with the database as a rebuildable serving layer.

## Decision

Postgres is the record of truth. A private repository holds the seed input and
the backups, never the authority.

## Rejected alternatives

- **Git repository as source of truth.** Rejected, but not for the reason first
  offered: 18MB of text is trivial for git, so scale was not the blocker.
- **The local folder as the backup.** True while all charts sit on one machine,
  false the moment a chart is added directly to production — and the loss would
  be silent during a restore.

## Consequences

- The load-bearing reason is phase 2: contributions write through `charts` /
  `chart_revisions` with a moderation queue and revision history, and git cannot
  be that write path.
- Backups become mandatory rather than optional — see
  [ADR-0004](0004-fly-compute-neon-postgres.md).
- `seed_songs` must fail loudly on unmatched titles instead of skipping silently.
- `ingest`'s `TRUNCATE` needs guarding against a malformed upstream snapshot.
