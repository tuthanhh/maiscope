# ADR-0024 — The song-id migration drops columns during a deploy, by exception

**Status:** accepted
**Date:** 2026-11-17

## Context

The deploy runs migrations automatically as Fly's `release_command`. The
standing rule (`fly.toml`, `bin/migrate.rs`, [ADR-0011](0011-manual-dispatch-deploy-first.md))
is expand-and-contract: additive migrations may ride a deploy, and
`DROP COLUMN` or `DROP TABLE` is a separate, deliberate, manual step,
because Postgres holds the only copy of chart text.

`20261117110000_song_uuid_id` swaps the primary key of `songs` from
`BIGINT` to `UUID` ([ADR-0023](0023-postgres-18-and-database-minted-song-ids.md)).
It copies `songs.id` and `sheets.song_id_fk` into new columns, then drops
the old ones, in one transaction. Nothing is lost: chart text hangs off
`sheets.id`, which does not change, and a test runs the migration down and
up with every sheet on its song and every chart on its sheet.

Two costs remain. Between the migration and the machine swap, the old
binary still decodes `songs.id` as an integer, so routes that read it fail
with 500s for those seconds. And a deploy that fails after migrating leaves
the old binary on the new schema.

## Decision

**This one migration rides the deploy, as a recorded exception.** The rule
stands for every other change; this ADR does not loosen it.

The deploy that carries it follows these steps:

1. Dispatch `backup-database.yml` and confirm it succeeded the same day.
2. Dispatch `deploy.yml` at a quiet hour.
3. Check `/api/v1/catalog` and one `/api/v1/songs/{id}` response.

If the deploy fails after the release command, recover forward: re-dispatch
`deploy.yml` once CI is green, so new code meets the new schema. Rolling
the schema back is a database restore: Neon's restore window, or the
dump from step 1 by the rehearsed procedure in
[`runbook.md`](../work/prod-data-and-infra/runbook.md).

## Rejected alternatives

**Expand and contract in three steps.** An additive migration adds the
UUID columns, kept in step by a trigger and the sync; code moves to them;
a manual step swaps the key and drops the old columns. For: follows the
rule exactly. Against: three deploys and a trigger for a key swap, which is
the awkward case for the pattern, to avoid seconds of errors on a
scale-to-zero app with no data at risk.

**A manual migration workflow.** Keep the migration out of the release
command and apply it from a new dispatch-only workflow in a planned window.
For: honours "drops are manual" literally. Against: a new production
workflow to build and rehearse, used once.

## Consequences

**Positive.** One reviewed migration and one deploy; the test already
proves the swap reversible.

**Negative.** A few seconds of 500s on song routes during the deploy.
Rollback is a restore, not a re-deploy. The exception sits beside a rule
it contradicts, so the next destructive migration must not cite it as
precedent; it needs its own decision.

**Neutral.** ADR-0011's progression towards automatic deploys is
unaffected: this deploy is dispatched by hand like every other.
