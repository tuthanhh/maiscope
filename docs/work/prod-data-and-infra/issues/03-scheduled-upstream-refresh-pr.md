# 03 — Scheduled upstream refresh, delivered as a PR

**What to build:** A cron workflow that fetches upstream `data.json`, and if it
differs from the vendored snapshot (issue 02), opens a pull request with the
updated file. Automated detection, human approval — the review gate is the part
with the value.

The diff is the early-warning system for the failure mode that silently loses
charts: an upstream song rename that breaks `seed_songs`' title matching.

**Blocked by:** 02 (obsolete)

**Status:** descoped — see Assessment

## Assessment (2026-09-13)

**Skip the workflow; keep the goal, get it much cheaper.** This ticket's whole
mechanism (fetch upstream, diff against a vendored snapshot, open a PR) rests on
ticket 02's vendored snapshot, which is now obsolete. But the *value* it names on
line 8 is real and currently unaddressed:

> The diff is the early-warning system for the failure mode that silently loses
> charts: an upstream song rename that breaks `seed_songs`' title matching.

**That failure is live, not hypothetical.** Measured against the real collection
on 2026-09-13: of 1927 song directories, **266 titles matched no catalog row and
291 songs were skipped entirely**, and the dev database holds ~278 charts attached
to sheets the current pack no longer matches. Chart text is already being silently
orphaned.

The cheap version: the signal is already in the database. `sync_catalog` stamps
`revision` on exactly the rows that changed, and logs vanished rows to
`deleted_songs` / `deleted_sheets`. A post-sync check can report "sheets that have
chart text but whose song title changed this revision" without vendoring anything
or opening a PR.

Proposed replacement scope:

- [ ] A check (binary or a step in `sync-catalog.yml`) reporting charts orphaned
      by a title change in the revision just applied
- [ ] Fails loudly, or at minimum annotates the run — a rename that drops charts
      must not be a quiet success
- [ ] No vendored snapshot, no data-repo workflow, no PR machinery

Note this overlaps the matcher work that `seed-charts.yml` needs in order to pass
without `--allow-unmatched`: both are about title matching being too strict for
the data. Worth doing as one piece.

### Also found: seeding is nondeterministic for duplicate titles

`bin/seed_songs.rs:116` builds its title → song-id map from
`SELECT id, title FROM songs WHERE title IS NOT NULL` with **no `ORDER BY`**, then
`HashMap::insert`s each row so the last write wins. **78 catalog titles are held by
more than one song** (1845 songs, 1767 distinct titles) — `Garakuta Doll Play` ×6,
`Wonderland Wars オープニング` ×6, `Reach For The Stars` ×3.

Postgres does not guarantee row order without `ORDER BY`, so which song a
duplicated title resolves to varies between runs. Observed directly on 2026-09-13:
two seeds of identical input into identically-bootstrapped databases produced 6571
and 6568 charts.

Consequence in production: a re-seed can silently reattach chart text to a
different song than the previous run picked. Chart text does not go missing, but it
moves, and nothing reports it.

- [ ] Make the mapping deterministic (`ORDER BY id`) so repeat runs agree
- [ ] Better: treat an ambiguous title as unmatched rather than guessing — a
      duplicated title cannot be resolved from `&title=` alone, and silently
      picking one is the failure this ticket is about. The song directory name or
      the version manifest (`.songs-layout.tsv`) can disambiguate.

- [ ] Scheduled workflow in the data repo (weekly is plenty)
- [ ] Fetches upstream, compares against the vendored snapshot
- [ ] No change → exits quietly, no PR, no noise
- [ ] Change → opens/updates a PR with the new snapshot
- [ ] PR body summarises the delta: songs added / removed / **retitled**, since
      retitles are the dangerous class
- [ ] Fails loudly if upstream is unreachable or returns a suspiciously small file
- [ ] Merging the PR does **not** auto-deploy — applying it is issue 04
