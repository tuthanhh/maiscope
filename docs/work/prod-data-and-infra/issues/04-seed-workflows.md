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

**Status:** in progress — `.github/workflows/seed-charts.yml` written 2026-09-13,
cannot run until the setup below exists

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

- [ ] Private chart-text repo created, holding the flat `songs/<song>/maidata.txt`
      tree. Set repo **variable** `CHART_DATA_REPO` to `owner/name`, and
      `CHART_DATA_SUBDIR` if the tree is not at `songs/`.
- [ ] Secret `CHART_DATA_TOKEN` — read access for seeding, write for the backup job.
- [ ] Secret `SEED_DATABASE_URL` — the lower-privilege role from ticket 01. The
      workflow falls back to `DATABASE_URL` so it works before that split, but
      then it seeds as owner.

**It will fail until the title matching is fixed.** The acceptance criterion below
— fail on anything unmatched, i.e. do not pass `--allow-unmatched` — is currently
unmeetable: 266 titles and 9 difficulties do not match. See ticket 03's assessment.
The workflow exposes `allow_unmatched` as an explicit, warned-about dispatch input
so the failure is a deliberate override rather than a silent default.

**Delivered by the workflow as written:**

- `workflow_dispatch` only, never push or merge; `environment: production` for a
  human approval gate; a `concurrency` group so two seeds cannot race on
  `catalog_meta.revision`
- Pre-mutation `pg_dump` uploaded as a run artifact (issue 05)
- Refuses to run when `COUNT(*) FROM songs` is 0, because seeding an unsynced
  catalog matches nothing and would look like a clean no-op
- Verifies the data tree is flat before touching the database, since a
  `<version>/<song>/` layout silently matches nothing
- Installs the Postgres 17 client; the runner's bundled 16 client refuses Neon's
  17 server

- [ ] `seed-charts.yml` — `workflow_dispatch`, runs `seed_songs` from the private
      data repo's `maidata.txt` tree
- [ ] `reload-catalog.yml` — `workflow_dispatch`, runs `ingest` from the vendored
      snapshot. Requires a typed confirmation input; states in its description
      that it truncates and invalidates delta sync.
- [ ] Both use the lower-privilege Neon role (issue 01), not owner
- [ ] Both take a `pg_dump` first (issue 05) and upload it as a run artifact
- [ ] `seed-charts` fails the run on anything unmatched — both titles and
      difficulties (`server-restructure` issue 09). This is the default: just
      don't pass `--allow-unmatched`, which is the only way to get exit `0`
      with unmatched entries.
- [ ] Run summary posts counts: seeded / already-up-to-date / unmatched
      difficulties / unmatched titles / skipped. Note `seeded` counts real
      writes only — a re-run over unchanged chart text reports `seeded 0` with
      everything under already-up-to-date, so a `0` here is normal, not a
      failure signal.
- [ ] Neither workflow can be triggered by a push or a merge
