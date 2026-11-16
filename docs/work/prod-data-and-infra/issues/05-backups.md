# 05 — Backups: scheduled `pg_dump` plus Neon's restore window

**What to build:** Postgres holds the only copy of chart text that exists outside
a laptop, on a free-tier database. Two mechanisms, because they cover different
failures:

- **Neon's restore window** handles "I dropped a column an hour ago." It does not
  survive account suspension, a free-tier policy change, or a project deleted by
  mistake.
- **Scheduled `pg_dump`** handles those, and outlives any single vendor account.

Explicitly **not** relying on "the laptop `songs/` folder is the backup." That is
true today with 16 charts and becomes false the first time a chart is added
directly to production — and the loss would be silent during a restore.

**Blocked by:** 01

**Status:** in-progress — `.github/workflows/backup-database.yml` written
2026-09-13; **it never ran until 2026-11-16, see Comments**; restore rehearsed
locally, not yet on Neon

## Assessment (2026-09-13)

**Destination decided: the private chart-data repo, under `backups/`.** The spec
offered "private repo, or object storage"; the sizes make the repo the obvious
pick, and it satisfies "outlives any single vendor account" without adding a third
provider or another credential.

Measured against a fully seeded database (1845 songs / 7348 sheets / 6571 charts):

| | size |
|---|---|
| `pg_dump -Fc -Z9` | **8.13MB** |
| live database | 37MB |

Revisit the destination if a dump passes ~100MB.

**Retention: last 7 daily, plus one per ISO week for 8 weeks**, enforced by
pruning files in the working tree. Git history still holds every dump ever
committed, which is the advantage of a repo over expiring artifact storage.

**Restore rehearsed for real, but locally.** On 2026-09-13 the 8.13MB dump was
restored with `pg_restore --no-owner` into a scratch database: no errors, and the
restored copy matched the source exactly (1845/7348/6571). Procedure written into
[`runbook.md`](../runbook.md), including the `--no-owner` requirement and the
delta-sync `409` consequence.

**Still outstanding:** the rehearsal must be repeated into a real Neon branch, and
Neon's actual free-tier restore window measured and recorded — that number is what
decides whether "we can roll back the data" is a fact. Both need Neon access.

**Scheduled an hour before `sync-catalog`** (18:00 vs 19:00 UTC), so the dump
captures the state *before* the nightly sync rather than after it.

- [x] Scheduled workflow running `pg_dump` (custom format, compressed)
- [x] Destination decided and documented: private repo, or object storage with
      credentials in repo secrets
- [x] Retention policy chosen (e.g. daily for 7, weekly for 8) and enforced
- [x] Pre-mutation dumps wired into issue 04's workflows
- [ ] **Restore rehearsed once for real** into a scratch Neon branch, and the
      procedure written into `runbook.md` — an untested backup is not a backup
      (rehearsed locally 2026-09-13, per the Assessment above; the box says
      Neon, and that has not happened yet)
- [ ] Dump size and duration recorded; revisit when charts reach ~1500 songs
      (size is recorded — 8.13MB against a fully seeded database, see the
      Assessment above — but no duration can be recorded yet: the workflow has
      never completed a run, see Comments. Record it from the first green run's
      "Dump the database" step)
- [x] Backup contents contain no secrets beyond the data itself — `pg_dump`
      (not `pg_dumpall`) writes schema and rows only; roles and their passwords
      are cluster-level and never enter a per-database dump. The connection
      string is read from a secret, used on the command line of one step, and
      never written to the archive or the step summary. Verified by reading
      the workflow, 2026-11-16.

## Comments

**2026-11-16 — the scheduled backup never ran.** Every one of the 60 nightly
runs since the workflow landed concluded `skipped`. There is no production
backup, and there never was one; the "last night's dump" recovery window
described in ticket 01 did not exist.

Cause: the job carried `if: vars.CHART_DATA_REPO != ''`, meant to skip cleanly
until the destination was configured. But `CHART_DATA_REPO` is an
**environment** variable on `production` (ticket 04 recorded exactly that), and
a job-level `if` is evaluated before the job's environment is bound — so
`vars.CHART_DATA_REPO` was empty at that point on every run, and the job
skipped. `seed-charts.yml` checks the same variable inside a step, where the
environment has resolved, which is why it worked and this did not.

Fix: the job-level `if` is gone and a first step checks the variable instead,
failing with `::error` rather than skipping. Deliberate change of behaviour: a
nightly failure gets read, and this nightly skip was not read for two months.

**To close the loop (needs a dispatch, not a local command):** run
`backup-database.yml` by hand once, confirm a `maiscope-YYYYMMDD.dump` lands in
the chart-data repo under `backups/`, and record the step duration here. Then
ticket 01's recovery-window claim becomes true.
