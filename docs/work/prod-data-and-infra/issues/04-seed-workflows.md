# 04 — Data workflows: catalog reload and chart seed, kept separate

**What to build:** In v1 there is **no write endpoint** — contributions were cut —
so the only way a chart enters production is direct database access. That
operation becomes a `workflow_dispatch` GitHub Action rather than a laptop
command.

Why a workflow rather than running it locally: production credentials stop living
on your disk next to your dev `.env`, and every run leaves an audit trail with a
timestamp, an actor and full output. "When did this chart change and why" becomes
answerable.

**Two separate workflows, deliberately.** `ingest` truncates the canonical tables
and bumps `last_full_reload_revision`, which invalidates every client's delta sync
(contract §3). It must not be firable by reflex alongside the harmless one.

**Blocked by:** 01 (role split), `server-restructure/issues/09-chart-revision-service-fn.md`.
No longer blocked by 02, which is obsolete.

**Status:** in-progress — setup complete and run for real against production
(2026-09-14); the title-matching gap below is the one thing still open

## Assessment (2026-09-13)

**`reload-catalog.yml` is obsolete.** It was specified to run `ingest` from a
vendored snapshot, truncating the canonical tables and bumping
`last_full_reload_revision`. `bin/ingest` no longer exists; `bin/sync_catalog`
replaced it and never truncates or deletes, and `sync-catalog.yml` already ships
it on a nightly schedule plus `workflow_dispatch`. The "keep the destructive one
separate so it is never fired by reflex" rationale has no destructive job left to
apply to. Only `seed-charts.yml` remains in scope.

**The 16GB problem does not exist.** This ticket assumed the chart-seed job needs
the song collection, which is ~16GB and cannot be checked out in Actions.
`seed_songs` only ever opens `maidata.txt` — never `track.mp3`, `pv.mp4` or
`bg.jpg`. Measured: **1928 files, 17.5MB raw, 3.9MB gzipped.** A private
chart-text repo is therefore an ordinary git repo: no LFS, no object storage.

**Setup this workflow still needs** (none of it is code):

- [x] Private chart-text repo created, holding the flat `songs/<song>/maidata.txt`
      tree. Set repo **variable** `CHART_DATA_REPO` to `owner/name`, and
      `CHART_DATA_SUBDIR` if the tree is not at `songs/`. Verified
      (2026-09-16): `CHART_DATA_REPO` = `tuthanhh/maiscope-chart-data` exists
      as a `production` environment variable; `CHART_DATA_SUBDIR` unset,
      using the `songs/` default.
- [x] Secret `CHART_DATA_TOKEN` — read access for seeding, write for the backup job.
      Confirmed present.
- [x] Secret `SEED_DATABASE_URL` — turned out unnecessary, not a gap. The
      role split (ticket 01, 2026-09-16) means the fallback `DATABASE_URL`
      **is now** `maiscope_app` (lower-privilege), not owner — so the
      workflow already runs as the intended role without a fourth secret.
      Superseded; see `runbook.md`.

**It will fail until the title matching is fixed.** The acceptance criterion below
— fail on anything unmatched, i.e. do not pass `--allow-unmatched` — is currently
unmeetable: 266 titles and 9 difficulties do not match. See ticket 03's assessment.
The workflow exposes `allow_unmatched` as an explicit, warned-about dispatch input
so the failure is a deliberate override rather than a silent default.

**Still true (verified 2026-09-16 against the last real run, `34815437919`,
2026-09-14):** `seeded 0 sheets (6559 already up to date), 9 unmatched
difficulties, 266 unmatched titles, 291 songs skipped entirely` — run with
`--allow-unmatched`. Charts are live and serving (confirmed by fetching one
from the production API), but this is a standing data-quality gap covered by
the explicit override, not a met acceptance criterion. Fixing the 266
mismatched titles is unscoped work, not part of this ticket.

**Delivered by the workflow as written:**

- `workflow_dispatch` only, never push or merge; `environment: production` for a
  human approval gate; a `concurrency` group so two seeds cannot race on
  `catalog_meta.revision`
- Pre-mutation `pg_dump` uploaded as a run artifact (issue 05)
- Refuses to run when `COUNT(*) FROM songs` is 0, because seeding an unsynced
  catalog matches nothing and would look like a clean no-op
- Verifies the data tree is flat before touching the database, since a
  `<version>/<song>/` layout silently matches nothing
- Installs the Postgres 18 client; the runner's bundled 16 client refuses Neon's
  18 server

- [x] `seed-charts.yml` — `workflow_dispatch`, runs `seed_songs` from the private
      data repo's `maidata.txt` tree
- **`reload-catalog.yml` is dropped**, not built. `bin/ingest` no longer exists —
      `apps/server/src/bin/` holds only `migrate`, `seed_chart`, `seed_songs`,
      `sync_catalog` — replaced by the differential `bin/sync_catalog`, which never
      truncates. See the header comment in `.github/workflows/seed-charts.yml`:
      "There is no truncating counterpart to this workflow... Ticket 04's
      `reload-catalog.yml` is obsolete for that reason."
- [x] Both use the lower-privilege Neon role (issue 01), not owner — satisfied
      via the `DATABASE_URL` fallback now pointing at `maiscope_app`, not a
      dedicated `SEED_DATABASE_URL` (see above)
- [x] Both take a `pg_dump` first (issue 05) and upload it as a run artifact
- [x] `seed-charts` fails the run on anything unmatched — both titles and
      difficulties (`server-restructure` issue 09). This is the default: just
      don't pass `--allow-unmatched`, which is the only way to get exit `0`
      with unmatched entries.
- [x] Run summary posts counts: seeded / already-up-to-date / unmatched
      difficulties / unmatched titles / skipped. Note `seeded` counts real
      writes only — a re-run over unchanged chart text reports `seeded 0` with
      everything under already-up-to-date, so a `0` here is normal, not a
      failure signal.
- [x] Neither workflow can be triggered by a push or a merge
