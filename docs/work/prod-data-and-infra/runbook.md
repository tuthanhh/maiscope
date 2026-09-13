# Runbook — production data

Operational procedures for the production database. Owned by
[ticket 01](issues/01-neon-provisioning-runbook.md) (bootstrap, roles) and
[ticket 05](issues/05-backups.md) (restore).

**Secret and variable inventory — names only, never values.**

| Name | Kind | Used by | Notes |
|---|---|---|---|
| `DATABASE_URL` | secret | `sync-catalog`, `deploy` (via Fly), `seed-charts`, `backup-database` | Currently the single `neondb_owner` role. Ticket 01 splits this. |
| `SEED_DATABASE_URL` | secret | `seed-charts`, `backup-database` | Not yet created. Lower-privilege role; both workflows prefer it and fall back to `DATABASE_URL`. |
| `FLY_API_TOKEN` | secret | `deploy` | |
| `CHART_DATA_TOKEN` | secret | `seed-charts`, `backup-database` | PAT or deploy key with read (seed) and write (backup) on the chart-data repo. |
| `CHART_DATA_REPO` | variable | `seed-charts`, `backup-database` | `owner/name` of the private chart-text repo. |
| `CHART_DATA_SUBDIR` | variable | `seed-charts` | Path to the song tree inside that repo. Defaults to `songs`. |

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

Rehearsed for real on 2026-09-13 against the 8.1MB dump of a fully seeded
database. `pg_restore` reported no errors and the restored copy matched the
source exactly: 1845 songs / 7348 sheets / 6571 charts.

```sh
# 1. Get the dump. Backups live in the chart-data repo under backups/,
#    named maiscope-YYYYMMDD.dump (custom format, compressed).

# 2. Confirm the archive is readable BEFORE destroying anything. This parses the
#    table of contents without needing a server, so a truncated file fails here.
pg_restore --list maiscope-YYYYMMDD.dump | head

# 3. Restore into a SCRATCH target first — a Neon branch, never the live
#    database. Verify, then promote.
pg_restore -d "$SCRATCH_DATABASE_URL" --no-owner maiscope-YYYYMMDD.dump

# 4. Verify the row counts match what the dump should contain.
psql -d "$SCRATCH_DATABASE_URL" -c "
  SELECT (SELECT COUNT(*) FROM songs)  AS songs,
         (SELECT COUNT(*) FROM sheets) AS sheets,
         (SELECT COUNT(*) FROM charts) AS charts,
         (SELECT revision FROM catalog_meta LIMIT 1) AS revision;"
```

`--no-owner` matters: the dump records `neondb_owner` as owner, and a restore
into a branch or a differently-named role fails on ownership statements without
it.

**After any restore, clients hold stale delta-sync state.** `catalog_meta.revision`
goes backwards relative to what clients last saw, so a client asking for changes
`since=<higher>` gets a `409`. That is the designed behaviour — the client
refetches `/catalog` in full — but expect a traffic spike.

## Known gaps

Tracked in [ticket 01](issues/01-neon-provisioning-runbook.md); recorded here
because they change how the procedures above should be read.

- **The app runs as `neondb_owner`.** Any logic bug has DDL rights on production.
  Splitting the roles is the highest-value item in ticket 01.
- **Migrations run through the pooler.** `MIGRATOR.run()` takes an advisory lock,
  and transaction-mode pooling does not guarantee the same backend across
  statements. It has worked, but it is not sound. Fix is a separate direct-endpoint
  URL the migrator prefers.
- **The current credential was pasted into a chat transcript** and should be
  rotated as part of doing ticket 01.
- **Neon's actual free-tier restore window is unverified.** Until it is measured
  and recorded here, "we can roll back the data" is an assumption rather than a
  fact — which is exactly why the `pg_dump` path exists.
