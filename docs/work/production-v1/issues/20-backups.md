# 20 — Backups: scheduled `pg_dump` plus Neon's restore window

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

**Blocked by:** 14

**Status:** todo

- [ ] Scheduled workflow running `pg_dump` (custom format, compressed)
- [ ] Destination decided and documented: private repo, or object storage with
      credentials in repo secrets
- [ ] Retention policy chosen (e.g. daily for 7, weekly for 8) and enforced
- [ ] Pre-mutation dumps wired into issue 19's workflows
- [ ] **Restore rehearsed once for real** into a scratch Neon branch, and the
      procedure written into `runbook.md` — an untested backup is not a backup
- [ ] Dump size and duration recorded; revisit when charts reach ~1500 songs
- [ ] Backup contents contain no secrets beyond the data itself
