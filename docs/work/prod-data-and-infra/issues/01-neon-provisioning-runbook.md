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
- [x] **Two roles**: an owner/migration role, and a lower-privilege role for
      seeding and for the app. The app must not run as owner. `maiscope_app`
      created with `SELECT, INSERT, UPDATE, DELETE` on `public` (plus
      `ALTER DEFAULT PRIVILEGES` so future migrations extend the grant
      automatically); `neondb_owner` kept for migrations only.
- [x] Connection string uses Neon's **pooled** endpoint for the app; the direct
      endpoint for migrations — `DATABASE_URL` (`maiscope_app`, pooled) and
      `MIGRATE_DATABASE_URL` (`neondb_owner`, direct) as two separate Fly
      secrets, wired in `bin/migrate.rs`
- [x] Pool `max_connections` reconciled with Neon's free-tier ceiling
      (`server-restructure` issue 03) — verified against [Neon's connection
      pooling docs](https://neon.com/docs/connect/connection-pooling)
      (2026-09-16): at the smallest compute size (0.25 CU, the floor a Neon
      project runs at), `max_connections` is 104, of which 7 are reserved for
      Neon's own superuser, leaving 97 available. The server's default of
      `4` (`DEFAULT_DATABASE_MAX_CONNECTIONS`, `config.rs`) is nowhere near
      that ceiling — no change needed. Not reconciled *up* either: 256MB/1
      shared CPU on a single Fly machine has no realistic path to needing
      more than 4 concurrent DB connections without evidence from real
      traffic first (same "raise only with evidence" rule as the VM sizing
      in `fly.toml`).
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
- [x] `runbook.md` in this directory: bootstrap steps, restore steps, role/secret
      inventory (names only, never values) — bootstrap and restore were
      already written (2026-09-13, rehearsed for real); secret inventory and
      "known gaps" updated (2026-09-16) to match the role split actually
      shipped, replacing the earlier `SEED_DATABASE_URL` plan that turned out
      unnecessary
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

Both defects above are now fixed (2026-09-16) — see below for how, including two
real failed deploys along the way.

**Fixing the role split, live, took three attempts.**

1. `DATABASE_URL` swapped to `maiscope_app` before it had any grants at all.
   Every deploy — local `flyctl deploy` and a real `deploy.yml` dispatch —
   failed identically: `migrate: failed: ... permission denied for schema
   public`. Release command aborted cleanly both times; the old machine kept
   serving throughout, exactly as `build-and-deploy` 03 verified it would.
2. Ran the `GRANT`/`ALTER DEFAULT PRIVILEGES` SQL as `neondb_owner`, added a
   new `MIGRATE_DATABASE_URL` Fly secret, and shipped `bin/migrate`'s
   preference for it over `DATABASE_URL`. Still failed — **same error, same
   pooler hostname.** Diagnostic in itself: `neondb_owner` owns the schema, so
   it can never get "permission denied for schema public" regardless of
   pooled vs. direct. The error proved the role in `MIGRATE_DATABASE_URL`'s
   *value* was still `maiscope_app` — Neon's Connection Details panel has a
   role dropdown independent of the pooled/direct toggle, and only the toggle
   had been changed.
3. Re-copied the connection string with the role dropdown explicitly set to
   `neondb_owner`. Real `deploy.yml` dispatch (run `35054247917`,
   2026-09-16T04:09:53Z): `release_command 080d16ef12e248 completed
   successfully`, healthcheck 200. First clean deploy since the split.

**Separately caught mid-fix:** the code change in step 2 was made on disk but
never committed before the first `deploy.yml` dispatch in step 1 — GitHub
Actions checks out git HEAD, so that run had none of the `MIGRATE_DATABASE_URL`
logic regardless of secrets. Local `flyctl` commands build from the working
directory and picked it up anyway, which is why local and CI runs briefly gave
different-looking failures for the same underlying cause. Fixed by committing
and merging before dispatching again.

**Correction to the credential-rotation note below:** re-checked against this
session's own transcript — every log line captured used the app's own
`redact_database_url`/`migrate.rs`'s `redact` (`postgres://***@host/db`), and
`gh api` secret calls returned only metadata, never values. Nothing was
exposed here. The note predates this session and still stands as a real,
separate to-do — not yet done.

Also note the credential currently in use was pasted into a chat transcript
(before this session) and should be rotated as part of doing this ticket
properly.
