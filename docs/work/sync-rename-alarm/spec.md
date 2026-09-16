# Spec — Rename alarm on the catalog sync

**Status:** planned
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Make an upstream rename visible to the maintainer on the run it happens, instead
of whenever a user notices a duplicate song.

## Problem

`catalog_sync` upserts `ON CONFLICT (song_id)` and `song_id` *is the title*, so
an upstream title correction is an id the table has never seen. The result is an
insert, not an update:

- the new row appears with no chart text — `hasChart: false`
- the old row fails the vanish sweep, is recorded in `deleted_songs`, and
  **is not deleted** (deleting a sheet would cascade to its chart text)
- so the catalog now lists the song twice: once playable under a dead title,
  once unplayable under the live one

[`api-rewrite`](../api-rewrite/spec.md) settled that renames are **not chased** —
a heuristic rematch on `image_name` + `artist` was rejected because 180
non-UTAGE songs share a jacket, and a false merge is silent and corrupting while
a missed rename is visible and recoverable. The plan of record is to fix it by
hand if noticed.

Nothing makes it noticeable. `sync-catalog.yml` runs on cron and its only step is
`cargo run --release --locked --bin sync_catalog` — a successful run is a green
check nobody opens, and the stats exist only in the job log.

[`production-v2`](../production-v2/spec.md) lists "how often upstream actually
renames a song" as a fact it does not trust. That number cannot be reasoned out;
it has to be instrumented and waited for. Doing it now means the answer exists
before `song-public-id` makes the assumption load-bearing.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Fail the sync run on the rename signature](issues/01-rename-signature-alarm.md) | Plus a step summary |

## Decisions that constrain this work

- **The signal already exists.** `SyncStats` computes both `songs_vanished`
  (`catalog_sync.rs:190`) and `songs_inserted`. Their coincidence in one run is
  the rename signature. This feature adds no detection logic, only reporting.
- **Loud, not automatic.** A tripped alarm exits non-zero so GitHub mails a
  failed run. Nothing is merged, renamed or reconciled automatically —
  `api-rewrite` rejected heuristic rematching, and this must not smuggle it back
  in.
- **False alarms are the accepted failure mode.** A song genuinely removed in
  the same run another is genuinely added trips it too. At maimai's release
  cadence that is a handful a year, and the alternative failure mode is silence.
- **Deployable against the current app.** No dependency on any other v2 work, so
  it ships on master before the branch is cut.

## Out of scope

Doing anything about a rename. Repointing chart text, merging rows, hiding the
tombstoned song — all of it stays manual, by decision.

## Done when

A sync run in which a song vanishes and a song appears exits non-zero, and every
run — tripped or not — shows its insert/update/vanish counts in the workflow
summary without opening the job log.
