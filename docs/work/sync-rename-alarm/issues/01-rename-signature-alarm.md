# 01 — Fail the sync run on the rename signature

**What to build:** `bin/sync_catalog` exits non-zero when one run both loses a
song and gains one, and `sync-catalog.yml` writes the run's stats to the job
summary.

The detection is a comparison, not an algorithm — `SyncStats` already carries
both counters. What is missing is anywhere for them to be seen.

**Blocked by:** None.

**Status:** done (2026-11-17)

- [x] `bin/sync_catalog` exits non-zero when `songs_vanished > 0 && songs_inserted > 0`
- [x] The failure message names the vanished `song_id`s and the inserted ones,
      so the job log shows the candidate pairing without a database session
- [x] The database transaction still **commits** — the sync's work is correct and
      keeping it is the point; only the process exit code changes. A rolled-back
      sync would re-trip the alarm on every subsequent run forever
- [ ] ~~An override for an acknowledged rename, so a known one does not fail every
      run until it is fixed by hand~~ not built: the alarm cannot repeat (see
      the comment below)
- [x] `sync-catalog.yml` writes insert/update/vanish counts to
      `$GITHUB_STEP_SUMMARY` on **every** run, tripped or not
- [x] Tests: the alarm trips when both counters are positive; it does not trip
      when only one is; the transaction is committed in both cases

> Renames are not chased — [`api-rewrite`](../../api-rewrite/spec.md) rejected
> heuristic rematching on measured collision risk. This ticket reports; it must
> not repoint, merge or reconcile anything.

## Comments

**2026-11-17: done.** `SyncStats` now carries the `song_id`s behind its
vanished and inserted counts, `rename_signature()` is the comparison, and
`summary_markdown()` is the job-summary table. `bin/sync_catalog` appends
that table to `$GITHUB_STEP_SUMMARY` on every run that reaches the database,
then exits **3** if the signature tripped (1 stays "failed, nothing
written"), after the transaction has committed.

**The override was not built, because the alarm fires once.** The vanish
sweep logs a song in `deleted_songs` the first time it goes missing and
never again, and the run commits. So the run after a rename sees no newly
vanished song and passes. An "acknowledged" switch would have nothing to
silence. A test pins the once-only behaviour. If that ever stops being
true, the override becomes necessary again.

Checked end to end on a throwaway database: sync, rename one `song_id` to
fake an upstream correction, sync again (exit 3, both songs named, summary
written), sync a third time (exit 0).

**Known gaps, left as found (review, 2026-11-17).** Two cases go quiet. A
rename that is later reverted (A to B, then B to A) does not trip on the
revert: A's row still exists, so it returns as an update, not an insert,
and the run sees one vanish and no insert. And re-running a failed sync
workflow in GitHub turns it green, which hides the alarm that already
fired. Catching the first would mean counting songs that come back from
`deleted_songs`, which is detection logic the spec rules out; it is the
maintainer's call. Failed and timed-out runs now write a one-line job
summary too, so no run leaves the summary empty.
