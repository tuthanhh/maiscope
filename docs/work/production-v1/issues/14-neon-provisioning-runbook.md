# 14 — Neon provisioning + production bootstrap runbook

**What to build:** Provision the production database and write down how it was
bootstrapped, because "prod starts clean" is a decision that must be repeatable.
Local Postgres data is never promoted; production is built by
`migrate` → `ingest` → `seed_songs` against an empty database.

Also resolve the facts the plan currently assumes rather than knows.

**Blocked by:** 13

**Status:** todo

- [ ] Neon project created; region chosen to match the Fly region (note the
      latency cost if they differ)
- [ ] **Two roles**: an owner/migration role, and a lower-privilege role for
      seeding and for the app. The app must not run as owner.
- [ ] Connection string uses Neon's **pooled** endpoint for the app; the direct
      endpoint for migrations
- [ ] Pool `max_connections` reconciled with Neon's free-tier ceiling (issue 04)
- [ ] **Verified and recorded here**: Neon free-tier restore/PITR window
- [ ] **Verified and recorded here**: whether free-tier branching allows a branch
      per PR (would let CI run `#[sqlx::test]` against real ephemeral Postgres)
- [ ] Measured and recorded: cold-start latency of the first query after autosuspend
- [ ] `runbook.md` in this directory: bootstrap steps, restore steps, role/secret
      inventory (names only, never values)
- [ ] Fly billing alerts configured — there is no hard spend cap
