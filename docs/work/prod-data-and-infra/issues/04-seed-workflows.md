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

**Blocked by:** 01, 02, `server-restructure/issues/09-chart-revision-service-fn.md`

**Status:** todo

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
