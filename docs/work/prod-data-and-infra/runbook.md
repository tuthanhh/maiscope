# Runbook — production data

Operational procedures for the production database. Owned by
[ticket 01](issues/01-neon-provisioning-runbook.md) (bootstrap, roles) and
[ticket 05](issues/05-backups.md) (restore).

**Secret and variable inventory — names only, never values.**

| Name | Kind | Used by | Notes |
|---|---|---|---|
| `DATABASE_URL` | secret | `sync-catalog`, `deploy` (via Fly, the app), `seed-charts`, `backup-database` | Role `maiscope_app` (2026-09-16), **pooled** endpoint. DML only (`SELECT`/`INSERT`/`UPDATE`/`DELETE`) — no DDL. |
| `MIGRATE_DATABASE_URL` | secret (Fly app only) | `deploy`'s `release_command` (`bin/migrate`) | Role `neondb_owner`, **direct** (non-pooled) endpoint. Migrations need DDL, and `MIGRATOR.run()`'s advisory lock needs a pinned backend connection that pooled mode does not guarantee. Falls back to `DATABASE_URL` if unset (`bin/migrate.rs`). |
| `FLY_API_TOKEN` | secret | `deploy` | |
| `CHART_DATA_TOKEN` | secret | `seed-charts`, `backup-database` | Fine-grained PAT, `Contents: Read and write` on the chart-data repo only. The backup pushes; seeding only reads. Replaced 2026-11-16 after the original turned out read-only ([ticket 05](issues/05-backups.md)). **Fine-grained tokens expire** — when this one does, both workflows fail at checkout with `Bad credentials`; regenerate, save here, redispatch. |
| `CHART_DATA_REPO` | variable | `seed-charts`, `backup-database` | `owner/name` of the private chart-text repo. |
| `CHART_DATA_SUBDIR` | variable | `seed-charts` | Path to the song tree inside that repo. Defaults to `songs`. |

`SEED_DATABASE_URL`, planned earlier as a third credential for seeding
specifically, was never created — `maiscope_app`'s single set of DML grants
already covers `sync-catalog` and `seed-charts` alongside the app itself, so a
fourth role would have added nothing.

## Bootstrap a database from empty

The documented sequence, in this order. Rehearsed end-to-end against a local
Postgres 17 on 2026-09-13 — timings from that run.

```sh
cargo run --release --bin migrate        # 5 migrations      ~3s
cargo run --release --bin sync_catalog   # 1845 songs        ~7s
cargo run --release --bin seed_songs     # 6571 charts      ~16s
```

Result: 1845 songs, 7348 sheets, 6571 charts, `catalog_meta.revision` advanced.

**Order is load-bearing.** `seed_songs` matches each `maidata.txt`'s `&title=`
against `songs.title`, so seeding before the catalog sync matches nothing and
skips every song — a silent no-op, not an error. `seed-charts.yml` guards against
this by refusing to run when `COUNT(*) FROM songs` is 0.

**`seed_songs` needs a flat song tree.** It reads `<songs-dir>/<song>/maidata.txt`
one level deep and does not recurse, so a `<version>/<song>/` layout silently
matches nothing. Use `scripts/flatten-songs.sh` (reversible via
`scripts/unflatten-songs.sh` and the manifest it writes).

In production, run these as workflows rather than locally:
`sync-catalog.yml`, then `seed-charts.yml`. Neither should be run from a laptop
holding production credentials.

## Restore from a dump

Rehearsed on Neon on 2026-11-16 with the first real backup: a fresh empty
database on a throwaway branch, 6.3s, and the four numbers below matched the
live `/sync/manifest` exactly (1879 songs / 7481 sheets / 6559 charts /
revision 10). Earlier rehearsed locally on 2026-09-13.

Backups have existed since 2026-11-16 (`backups/maiscope-20261116.dump` was the
first). Before that date `backup-database.yml` skipped every run — see
[ticket 05](issues/05-backups.md) — so there is nothing to restore from for
any earlier point.

```sh
# 1. Get the dump. Backups live in the chart-data repo under backups/,
#    named maiscope-YYYYMMDD.dump (custom format, compressed).
gh api -H "Accept: application/vnd.github.raw" \
  repos/tuthanhh/maiscope-chart-data/contents/backups/maiscope-YYYYMMDD.dump \
  > maiscope-YYYYMMDD.dump

# 2. Confirm the archive is readable BEFORE destroying anything. This parses the
#    table of contents without needing a server, so a truncated file fails here.
pg_restore --list maiscope-YYYYMMDD.dump | head

# 3. Restore into a SCRATCH target first — a fresh, EMPTY database on a Neon
#    branch, role neondb_owner, direct (non-pooled) endpoint. Never the live
#    database, and not the branch's copied database either: an empty target is
#    what proves the dump alone rebuilds everything. Keep the string in a
#    variable read without echo; it goes in no file and no history.
read -rs SCRATCH_DATABASE_URL
pg_restore -d "$SCRATCH_DATABASE_URL" --no-owner --no-privileges maiscope-YYYYMMDD.dump

# 4. Verify the row counts against what production reports right now.
psql -d "$SCRATCH_DATABASE_URL" -c "
  SELECT (SELECT COUNT(*) FROM songs)  AS songs,
         (SELECT COUNT(*) FROM sheets) AS sheets,
         (SELECT COUNT(*) FROM charts) AS charts,
         (SELECT revision FROM catalog_meta LIMIT 1) AS revision;"
curl -s https://maiscope-api.fly.dev/api/v1/sync/manifest

# 5. Delete the branch in the Neon console; unset the variable; delete the file.
```

`--no-owner` matters: the dump records `neondb_owner` as owner, and a restore
into a branch or a differently-named role fails on ownership statements without
it. `--no-privileges` skips the `GRANT`s to `maiscope_app` the archive carries;
they would apply (Neon roles are project-wide) but a restore is about the data.

Promoting a restored branch to serve production is a Neon console operation
(branch → *Set as default*, or restore the main branch from it) followed by a
`deploy.yml` dispatch so the release command re-checks migrations. Not
rehearsed; the data path is.

**After any restore, clients hold stale delta-sync state.** `catalog_meta.revision`
goes backwards relative to what clients last saw, so a client asking for changes
`since=<higher>` gets a `409`. That is the designed behaviour — the client
refetches `/catalog` in full — but expect a traffic spike.

## Known gaps

Tracked in [ticket 01](issues/01-neon-provisioning-runbook.md); recorded here
because they change how the procedures above should be read.

**Resolved (2026-09-16):** the app no longer runs as `neondb_owner` (split
into `maiscope_app`, DML-only, and `neondb_owner` for migrations only — see
the secret table above), and migrations no longer run through the pooler
(`MIGRATE_DATABASE_URL` pins the direct endpoint). Verified against a real
`deploy.yml` dispatch, not just reasoned about — see ticket 01's Comments for
the three-attempt debugging trail.

Still open:

- **The current credential was pasted into a chat transcript** (predates
  2026-09-16's work) and should be rotated.
- **Resolved (2026-11-16):** restore rehearsed into a real Neon branch — see
  *Restore from a dump* above and [ticket 05](issues/05-backups.md). Neon's
  free-tier PITR window is 6 hours (ticket 01, 2026-09-16); the nightly
  `pg_dump` is the recovery path for anything older, and it now exists.
