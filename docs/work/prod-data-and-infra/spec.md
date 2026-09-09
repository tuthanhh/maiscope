# Spec — Production data and infrastructure

**Status:** planned
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Provision the production database, stop `ingest` from depending on a third party's
bucket at runtime, move data mutation off laptops into audited workflows, and make
the backup real. Postgres is the source of truth for chart text
([ADR-0001](../../adr/0001-postgres-source-of-truth-for-charts.md)) on a free-tier
database, which is what makes this feature load-bearing rather than housekeeping.

## Problem

`bin/ingest.rs` hardcodes a third party's CloudFront URL **and** `TRUNCATE`s the
canonical tables before reloading from it. An upstream outage, a malformed file, or
a truncated response can wipe the catalog and reload garbage in one command.

Separately, v1 has no write endpoint — contributions were cut
([ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md)) — so the only way a
chart enters production is direct database access, which today means production
credentials sitting on a laptop next to a dev `.env`.

And the umbrella spec's "verify before relying on" list is not decoration: Neon's
actual restore window decides whether "roll back the data" is a true statement.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Neon provisioning + bootstrap runbook](issues/01-neon-provisioning-runbook.md) | Also owns the umbrella spec's unverified facts |
| 02 | [Vendor the upstream `data.json` snapshot](issues/02-vendor-upstream-datajson.md) | Independent of the server restructure — can start any time |
| 03 | [Scheduled upstream refresh, delivered as a PR](issues/03-scheduled-upstream-refresh-pr.md) | Automated detection, human approval |
| 04 | [Catalog reload and chart seed, kept separate](issues/04-seed-workflows.md) | |
| 05 | [Backups: scheduled `pg_dump` plus Neon's restore window](issues/05-backups.md) | |

## Decisions that constrain this work

- **Production starts clean.** Local Postgres data is never promoted; production is
  built by `migrate` → `ingest` → `seed_songs` against an empty database, and that
  sequence is written down because it must be repeatable.
- **Compute and data are separate** ([ADR-0004](../../adr/0004-fly-compute-neon-postgres.md)),
  so a redeploy can never touch data.
- **The private data repo is seed input and backup, never the authority.** One
  artifact serves both roles: the vendored snapshot plus the `maidata.txt` tree.
- **`ingest` truncates** (umbrella spec, rule 3). The catalog-reload job and the
  chart-seed job stay separate workflows so the destructive one is never fired by
  reflex. Reload also bumps `last_full_reload_revision`, invalidating every client's
  delta sync.
- **The app does not run as owner.** Two Neon roles: owner for migrations,
  lower-privilege for seeding and for the app.
- **An untested backup is not a backup.** Ticket 05 rehearses a restore for real
  into a scratch branch before it is called done.

## Facts this feature must resolve

Carried from the umbrella spec, owned by ticket 01 as acceptance criteria:

- Neon's actual free-tier restore/PITR window.
- Whether the free tier allows a branch per PR — that would let CI run
  `#[sqlx::test]` against real ephemeral Postgres.
- Cold-start latency of the first query after autosuspend.

## Out of scope

Any write endpoint or moderation queue — phase 2. Audio hosting, permanently
([ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md)).

## Done when

Production is provisioned and its bootstrap is written into `runbook.md`, `ingest`
reads a vendored file with sanity checks before it truncates, upstream drift arrives
as a reviewable PR that names retitled songs, both data workflows are
`workflow_dispatch`-only and dump before mutating, and a restore has been performed
once for real.
