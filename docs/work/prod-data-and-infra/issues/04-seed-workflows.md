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

**It failed until the title matching was fixed (2026-11-17, below).** The
acceptance criterion — fail on anything unmatched, i.e. do not pass
`--allow-unmatched` — was unmeetable while 266 titles and 9 difficulties did
not match. The workflow exposes `allow_unmatched` as an explicit, warned-about
dispatch input so the failure is a deliberate override rather than a silent
default. After the fix, 11 titles remain, every one a song the catalog does
not hold; see the comment for the choice that leaves.

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

**Matching fixed, and a worse bug found under it (2026-11-17).** The 266
unmatched titles were classified against the dev catalog
(`seed_songs --allow-unmatched` over the local 1927-folder sample):

| Class | Folders | Now |
|---|---|---|
| ` [DX]` / ` [ST]` suffix on the title, base title in the catalog | 81 | matched; suffix names the chart type |
| `[宴…]` UTAGE folders (`&lv_7=`, charts in slot 7) | 164 | counted and skipped, not failed — `catalog-difficulties` 03 |
| `&title=` cut at `&` in the sample (`LOVE` for *LOVE & JOY*), folder name intact | 5 | matched via the folder name |
| full-width `～（！＃＆` or case or whitespace differs from upstream | 9 | matched via a loose fold (NFKC, case, whitespace) |
| song not in the catalog (removed, or another region: 回马枪, 飞行志…) | 11 | unmatched, correctly |

The 9 "unmatched difficulties" were the same 3 songs seen through the wrong
chart type (Ievan Polkka's folder is its DX set; the catalog's standard set
has fewer difficulties), and are 0 now.

**The worse bug.** `seed_songs` keyed catalog sheets by *(song, difficulty)*
with no chart type. 81 catalog songs have a standard and a DX set with the
same difficulty names, so for those the map kept whichever sheet Postgres
returned last, and the folder's charts — the standard ones, since the
` [DX]` folders were the unmatched ones — were written to it. In dev, the DX
master of *39* held the standard master chart. Production ran the same code
once, so the same is true there for some subset of those 81 songs: a reader
opening a DX sheet sees the standard chart. Fixed: a folder now resolves to
one *(song, type)* — the suffix decides when present, otherwise the
maidata's `&lv_N=` levels are scored against each type's sheet levels and
only a unique best match seeds; a tie is reported as ambiguous and seeds
nothing (ADR-0012's reject-don't-guess, applied to data). Thirteen unit
tests on the resolver. On dev the re-run rewrote **384 sheets** — the 81
songs' DX sets getting their real charts, plus everything the suffix and
fold matching recovered. Expect the same order of magnitude on the next
production dispatch; the pre-mutation `pg_dump` the workflow takes covers
the rollback.

**Decision left for the maintainer — the last 11.** They are not matching
failures, so no code change recovers them. Two ways to make the criterion
hold without the override:

- **A. Drop the 11 folders from the chart-data repo** (recommended: the
  catalog is upstream's, and a chart for a song it does not list has no
  sheet to attach to). The workflow then exits 0 without `--allow-unmatched`.
- **B. Keep the override**, and treat the summary's `unmatched titles` count
  as the thing to watch: 11 is the known floor, anything above it is a
  regression.

Until A is done, dispatch with `allow_unmatched` as before.

**Dispatch on 2026-11-16 (UTC) failed before seeding: the data repo has no
`main`.** The `data_ref` input defaulted to `main` since the workflow was
written; the chart-data repo's branch is `master` (the September run must
have been dispatched with it typed in). `actions/checkout` reports a missing
branch as `The process '/usr/bin/git' failed with exit code 1`, three times,
with nothing on stderr — git's wildcard fetch of a non-existent branch exits
1 silently (reproduced locally). The backup workflow never hit this because
it passes no `ref` and gets the default branch. Fixed the same way: the
input now defaults to empty, meaning the data repo's default branch, and a
step before checkout resolves a non-empty ref through the API so the error
names the ref. Redispatch with `data_ref` left empty.

**Production reseeded 2026-11-16 (UTC), run `43929322794`, on `1e1e448`:**

```
seeded 433 sheets (6502 already up to date), 0 unmatched difficulties, 12 unmatched titles, 37 songs skipped entirely, 164 UTAGE folders not seedable yet, 14 matched loosely
```

The 433 are the DX sets of the dual-type songs receiving their own charts
plus the suffix- and fold-recovered folders; dev predicted 384 on a slightly
smaller catalog. Twelve unmatched against production's catalog versus
eleven against dev's — one title differs between the two catalogs; not
chased.

**The last-folders decision is deferred to v2.0** (maintainer, 2026-11-17):
until then `seed-charts.yml` is dispatched with `allow_unmatched` and
**12** is the known floor for `unmatched titles` — a run reporting more has
regressed. The acceptance criterion stays unticked on purpose; the
`community-charts` work decides what a chart for a song outside the catalog
even is.
