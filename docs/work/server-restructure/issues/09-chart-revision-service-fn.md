# 09 — Extract "apply a chart revision"; make `seed_songs` fail loudly

**What to build:** Two related changes to the chart write path.

**1. Extract the domain function.** "Apply a chart revision" — upsert `charts`,
append to `chart_revisions`, bump `revision` — should exist once as a service
function. `seed_songs` calls it now; phase 2's `POST /contributions/{id}/approve`
calls it later. This is the part of the write path that genuinely carries forward
to community contributions (a token-guarded admin endpoint would not — its auth
model is throwaway and it has no review queue).

**2. Stop failing silently.** `bin/seed_songs.rs:8,14` matches maidata `&title=`
against `songs.title` NFC-normalized, and unmatched charts are *skipped
silently*. When upstream renames a song, the chart quietly stops loading and
nobody finds out. Under `prod-data-and-infra` issue 04 this runs unattended in CI, which makes silence
much worse.

**Blocked by:** 04

**Status:** todo

- [ ] `apply_chart_revision(&mut tx, sheet_id, format, content, hash, …)` service
      function, transactional, in its own module (not in `bin/`)
- [ ] `seed_songs` rewritten to call it — no inline SQL
- [ ] Unmatched titles collected and reported at the end, with the offending title
- [ ] Non-zero exit when any chart failed to match, unless `--allow-unmatched`
- [ ] Summary line keeps counts (seeded / warnings / skipped) for CI logs
- [ ] Unit test: applying a revision twice appends two `chart_revisions` rows and
      leaves one `charts` row (the `UNIQUE (sheet_id, format)` invariant)
