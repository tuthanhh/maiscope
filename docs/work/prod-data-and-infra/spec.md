# Spec — Production data and infrastructure

**Status:** active
**Milestone:** v1.0
**Parent:** [`production-v1`](../production-v1/spec.md)

Provision the production database, move data mutation off laptops into audited
workflows, and make the backup real. Postgres is the source of truth for chart text
([ADR-0001](../../adr/0001-postgres-source-of-truth-for-charts.md)) on a free-tier
database, which is what makes this feature load-bearing rather than housekeeping.

## Problem

`bin/ingest.rs` hardcoded a third party's CloudFront URL **and** `TRUNCATE`d the
canonical tables before reloading from it, so an upstream outage, a malformed file
or a truncated response could wipe the catalog and reload garbage in one command.
That binary is gone. [`catalog-sync`](../catalog-sync/spec.md) replaced it with
`bin/sync_catalog`, which diffs upstream into the canonical tables and never
deletes ([ADR-0010](../../adr/0010-differential-catalog-sync.md)). Tickets 02 and
03 existed to make the truncate survivable, so both are dropped.

The other half of the problem stands. v1 has no write endpoint — contributions
were cut ([ADR-0002](../../adr/0002-chart-data-only-no-audio-hosting.md)) — so the
only way a chart enters production is direct database access, which today means
production credentials sitting on a laptop next to a dev `.env`.

And the umbrella spec's "verify before relying on" list is not decoration: Neon's
actual restore window decides whether "roll back the data" is a true statement.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Neon provisioning + bootstrap runbook](issues/01-neon-provisioning-runbook.md) | Also owns the umbrella spec's unverified facts |
| 02 | [Vendor the upstream `data.json` snapshot](issues/02-vendor-upstream-datajson.md) | **Dropped.** Nothing truncates any more, so there is nothing to make survivable |
| 03 | [Scheduled upstream refresh, delivered as a PR](issues/03-scheduled-upstream-refresh-pr.md) | **Dropped.** Its one surviving goal is now [`sync-rename-alarm`](../sync-rename-alarm/spec.md) |
| 04 | [Catalog reload and chart seed, kept separate](issues/04-seed-workflows.md) | The reload half is obsolete; only `seed-charts.yml` is in scope |
| 05 | [Backups: scheduled `pg_dump` plus Neon's restore window](issues/05-backups.md) | |

## Decisions that constrain this work

- **Production starts clean.** Local Postgres data is never promoted; production is
  built by `migrate` → `sync_catalog` → `seed_songs` against an empty database, and
  that sequence is written down because it must be repeatable.
- **Compute and data are separate** ([ADR-0004](../../adr/0004-fly-compute-neon-postgres.md)),
  so a redeploy can never touch data.
- **The private data repo is seed input and backup, never the authority.** It holds
  the `maidata.txt` tree and nothing else. The upstream catalog is fetched on each
  sync, not vendored.
- **The catalog job and the chart-seed job stay separate workflows.** Neither
  truncates now, but they write different tables on different cadences:
  `sync-catalog.yml` runs on cron because an unattended differential sync is safe,
  and `seed-charts.yml` is dispatch-only because chart text is the one asset only
  this project holds. Collapsing them would put chart writes on a schedule nobody
  watches.
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

Production is provisioned and its bootstrap is written into `runbook.md`,
`seed-charts.yml` is `workflow_dispatch`-only and dumps before mutating, and a
restore has been performed once for real into a scratch branch.

The reviewable-PR half of this feature is gone: `sync_catalog`'s `sanity_check`
rejects an empty or implausibly shrunken payload before any write, and naming
retitled songs moved to [`sync-rename-alarm`](../sync-rename-alarm/spec.md).
