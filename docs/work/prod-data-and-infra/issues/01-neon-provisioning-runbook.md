# 01 — Neon provisioning + production bootstrap runbook

**What to build:** Provision the production database and write down how it was
bootstrapped, because "prod starts clean" is a decision that must be repeatable.
Local Postgres data is never promoted; production is built by
`migrate` → `ingest` → `seed_songs` against an empty database.

Also resolve the facts the plan currently assumes rather than knows.

**Blocked by:** `build-and-deploy/issues/03-migrate-binary-and-flytoml.md`

**Status:** todo (inherits a partly-provisioned state — see Comments)

- [x] Neon project created; region chosen to match the Fly region (note the
      latency cost if they differ) — `ap-southeast-1`, paired with Fly `sin`
- [ ] **Two roles**: an owner/migration role, and a lower-privilege role for
      seeding and for the app. The app must not run as owner.
- [ ] Connection string uses Neon's **pooled** endpoint for the app; the direct
      endpoint for migrations
- [ ] Pool `max_connections` reconciled with Neon's free-tier ceiling (`server-restructure` issue 03)
- [x] **Verified and recorded here**: Neon free-tier restore/PITR window —
      **6 hours** of change history (up to 1 GB-month), confirmed against
      [Neon's own plans page](https://neon.com/docs/introduction/plans)
      (2026-09-16), not a third-party aggregator. Combined with the nightly
      `pg_dump` backup (`prod-data-and-infra` 05), the true recovery window is
      "last night's dump" for anything older than 6 hours, not just PITR.
- [x] **Verified and recorded here**: whether free-tier branching allows a branch
      per PR — **yes**, Free plan allows **10 branches/project**, comfortably
      above the concurrent-PR count a solo repo ever has open. Not implemented
      here (would let CI run `#[sqlx::test]` against real ephemeral Postgres) —
      tracked as a future enhancement, not blocking this ticket.
- [ ] Measured and recorded: cold-start latency of the first query after autosuspend
- [ ] `runbook.md` in this directory: bootstrap steps, restore steps, role/secret
      inventory (names only, never values)
- [ ] Fly billing alerts configured — there is no hard spend cap


## Comments

**Neon was provisioned ad-hoc on 2026-09-13** while closing `build-and-deploy` 03,
so this ticket starts from a partly-built environment rather than a blank slate.
What exists, and where it already diverges from this ticket's requirements:

| Requirement | Actual |
|---|---|
| Project created, region matching Fly | **Done** — `ap-southeast-1`, paired with Fly `sin` |
| Migrations applied | **Done** — all 5, via `bin/migrate` as Fly's release command |
| Two roles: owner for migrations, lower-privilege for the app | **Not done** — a single `neondb_owner` does everything, so the app runs as owner |
| Pooled endpoint for the app, direct for migrations | **Not done** — one `DATABASE_URL` secret, pointing at the **pooled** endpoint, is used by both the server and the release command |
| Pool `max_connections` reconciled with the free-tier ceiling | **Not done** — still the default 4 |
| Bootstrap runbook written down | **Not done** — the sequence was run by hand from a laptop |

Two of those are worth treating as defects rather than pending work:

- **The app runs as owner.** Any SQL-injection or logic bug has DDL rights on
  production. Splitting the roles is the single highest-value item here.
- **Migrations run through the pooler.** `MIGRATOR.run()` takes a Postgres advisory
  lock, and transaction-mode pooling does not guarantee the same backend across
  statements. It has worked so far, but it is not sound. The fix is a separate
  direct-endpoint URL for the migrator — `bin/migrate` would prefer a
  `MIGRATE_DATABASE_URL` when set.

Also note the credential currently in use was pasted into a chat transcript and
should be rotated as part of doing this ticket properly.
