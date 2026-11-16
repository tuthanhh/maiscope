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

**Status:** done — 2026-11-16. Workflow written 2026-09-13, never ran until
the guard fix; first backup committed and restore rehearsed on Neon the same
day, see Comments

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
- [x] **Restore rehearsed once for real** into a scratch Neon branch, and the
      procedure written into `runbook.md` — 2026-11-16, see Comments. Fresh
      empty database on a throwaway branch, `pg_restore --no-owner
      --no-privileges`, **6.3s**, and every number matched the live
      `/sync/manifest`: 1879 songs / 7481 sheets / 6559 charts / revision 10
- [x] Dump size and duration recorded; revisit when charts reach ~1500 songs.
      First real run, 2026-11-16 (`43833756112`): **8,388,287 bytes, 26s** for
      the "Dump the database" step (`pg_dump -Fc -Z9` from a GitHub runner to
      Neon, including the version probe). Whole job 43s. Against 8.13MB
      measured locally on 2026-09-13, so the catalog has barely moved
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

**2026-11-16, later — first dispatch, half a success.** Run `43833756112`:
every step up to and including the commit passed; the push to the chart-data
repo was refused:

```
remote: Write access to repository not granted.
fatal: unable to access 'https://github.com/tuthanhh/maiscope-chart-data/': The requested URL returned error: 403
```

`CHART_DATA_TOKEN` can read the repo (checkout succeeded, and `seed-charts`
has always worked) but cannot write it. The runbook's secret table said "read
(seed) and write (backup)"; the write half was never true, and nothing
exercised it until now because the job had never reached the push.

**The dump is not lost.** "Upload the dump as an artifact" runs `if: always()`
and succeeded: `maiscope-dump-20261116`, 8,310,047 bytes compressed, on the
run page, **expires 2026-12-16**. That is the first production backup that has
ever existed. It is in artifact storage, not the repo, so the retention policy
does not apply to it and it disappears in 30 days.

**To close the loop (both need the maintainer):**

1. Give `CHART_DATA_TOKEN` write access to `tuthanhh/maiscope-chart-data` —
   a fine-grained PAT with *Contents: Read and write* on that one repo, or a
   deploy key with write enabled — and update the `production` environment
   secret. Read-only is enough for `seed-charts`; the backup is the only
   writer.
2. Dispatch `backup-database.yml` again. Success is a
   `backups/maiscope-YYYYMMDD.dump` commit in the chart-data repo, not a green
   run. Until then, download the artifact and keep it somewhere that is not
   this laptop's `songs/` folder.

The push step now names this failure and the artifact fallback in its error,
so the next 403 reads as "fix the token", not as a git mystery.

**2026-11-16, 09:17Z — loop closed.** Run `43837972537`: success end to end.
`backups/maiscope-20261116.dump` (8,388,287 bytes) is committed in
`tuthanhh/maiscope-chart-data`. Dump step 25s, whole job just over 3 minutes
including runner pickup. `CHART_DATA_TOKEN` was replaced at 09:16:50Z with a
token that can write; the run in between (`43835668238`) failed at checkout
with `Bad credentials` because the old token had already been invalidated
before the new value was saved. Harmless, and a useful reminder: regenerate
the token *after* noting where it is used, then save, then dispatch.

**A production backup now exists**, and the nightly schedule will run from
here. Ticket 01's "last night's dump" recovery window is true from tonight.

**2026-11-16, 16:30 +07 — restore rehearsed on Neon.** The same dump,
downloaded from the chart-data repo (`gh api` with the raw accept header,
8,388,287 bytes, `pg_restore --list` reads 118 TOC entries), restored into a
**fresh empty database** on a throwaway branch `restore-rehearsal` — empty on
purpose, so the dump alone had to rebuild the schema and the data, with no
branch-copied rows to hide a gap.

```
pg_restore -d "$SCRATCH_DATABASE_URL" --no-owner --no-privileges maiscope-20261116.dump
  6.309s wall

 songs | sheets | charts | revision
  1879 |   7481 |   6559 |       10

GET https://maiscope-api.fly.dev/api/v1/sync/manifest
  {"counts":{"charts":6559,"sheets":7481,"songs":1879},"revision":10,"updateTime":"2026-10-30"}
```

Client `pg_restore` 18.6 against server 18.7, dumped by `pg_dump` 18 on the
runner (`Dump Version: 1.16-0`). `--no-privileges` added to the runbook's
`--no-owner`: the archive carries `GRANT`s to `maiscope_app`, which would
apply (roles are project-wide in Neon) but are not what a rehearsal is
testing. Branch deleted afterwards.

**What this proves and what it does not.** The nightly dump restores to an
exact copy of the catalog in seconds; "we can roll the data back" is now a
fact with a date. It does not cover the Neon PITR path (6-hour window,
ticket 01), which was not exercised and does not need to be — the dump is the
recovery path for anything older than six hours anyway.
