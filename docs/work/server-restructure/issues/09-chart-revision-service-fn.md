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

**Status:** done

- [x] `apply_chart_revision(&mut tx, sheet_id, format, content, hash, …)` service
      function, transactional, in its own module (not in `bin/`)
- [x] `seed_songs` rewritten to call it — no inline SQL
- [x] Unmatched titles collected and reported at the end, with the offending title
- [x] Non-zero exit when any chart failed to match, unless `--allow-unmatched`
- [x] Summary line keeps counts (seeded / warnings / skipped) for CI logs
- [x] Unit test: applying a revision twice appends two `chart_revisions` rows and
      leaves one `charts` row (the `UNIQUE (sheet_id, format)` invariant)

## Comments

**No `lib.rs` existed before this ticket.** `src/bin/seed_songs.rs` is a
separate crate root from `main.rs` — it cannot see anything declared in
`main.rs`'s module tree, which is exactly why `upsert_chart` used to live
duplicated inline there. Added `apps/server/src/lib.rs` (`pub mod
chart_revision;`) so both the `seed_songs` binary and (later) the phase-2
approve handler in the main `server` binary can import the same function.
Nothing else moved into the library — every other existing module stays
private to `main.rs`.

**"Bump revision" means stamping the sheet with a fresh number drawn from
`catalog_meta.revision`, not incrementing `sheets.revision` on its own.**
`sync_delta`'s `WHERE s.revision > $1` comparison only works if every row's
revision lives in the same numbering space `catalog_meta.revision` uses —
`bin/ingest.rs` already establishes that convention (`new_revision =
previous_revision + 1`, stamped onto every changed row in the same
transaction); `apply_chart_revision` mirrors it exactly rather than
inventing an independent counter. Verified live: `catalog_meta.revision`
before/after a real `apply_chart_revision` call via `GET /sync/manifest`.

**Idempotent on unchanged content** — not explicitly asked for by the
ticket text, but required for `seed_songs` (a repeatable bulk re-seed tool)
to be safe to run more than once: re-seeding byte-identical chart text
early-returns before touching `charts`, `chart_revisions`, or any revision
counter, checked against the stored `hash`. Verified live, not just by the
unit test: `cargo run --bin seed_songs` three times in a row against the
real `songs/` directory — 66 charts on the first run, `catalog_meta.revision`
held exactly flat across the second and third.

**CLI parsing** (`seed_songs.rs`) switched from "argument 1 is always the
songs dir" to scanning all args for `--allow-unmatched` vs. a path, since
position could no longer imply meaning once there were two independent
things to configure. Verified live with a throwaway fixture directory
(`/tmp/fixture-songs`, not the real `songs/` dir or real catalog data):
exit `1` without the flag on an unmatched title, exit `0` with it.

Full walkthrough with code, including the `&mut *tx` vs `&mut **tx`
transaction-executor deref distinction (a real mistake caught by the
compiler while writing this, not just a hypothetical), in
`.claude/study/chart-revision-service-fn.md`.
