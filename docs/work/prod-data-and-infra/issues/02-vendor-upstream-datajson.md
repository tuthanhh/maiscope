# 02 — Vendor the upstream `data.json` snapshot

**What to build:** `bin/ingest.rs:17` hardcodes
`https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json` — a third party's bucket —
and `bin/ingest.rs:182` `TRUNCATE`s the canonical tables before reloading from it.
That combination means an upstream outage, a malformed file, or a truncated
response can wipe the catalog and reload garbage in one command.

Change `ingest` to read a **vendored snapshot file**. Refreshing upstream becomes
a reviewable commit (issue 03), so a rename or a shrink is visible in `git diff`
*before* it is applied — which is also the earliest place to catch the title
mismatches that make `seed_songs` drop charts (`server-restructure` issue 09).

The snapshot lives in the private data repo alongside the chart text, so one
artifact serves as seed input and as backup.

**Blocked by:** None (independent of the server restructure)

**Status:** obsolete — see Assessment

## Assessment (2026-09-13)

**Skip this ticket.** Every line of its rationale is about `bin/ingest.rs`, which
no longer exists: it was replaced by the differential `bin/sync_catalog`. The
failure this ticket exists to prevent is now architecturally impossible.

| This ticket's concern | Status under `sync_catalog` |
|---|---|
| `TRUNCATE`s before reloading | Never truncates, never deletes — enforced by the module doc in `catalog_sync.rs` |
| An outage or malformed file "wipes the catalog and reloads garbage" | Cannot: nothing is deleted, and a failed run rolls back one transaction |
| "Sanity checks before `TRUNCATE`: parses, count within a sane delta, non-empty" | Already implemented as `catalog_sync::sanity_check` — rejects an empty payload and any payload below 90% of the current row count, before any write |

The vendored-snapshot machinery, the private data repo for the snapshot, and the
`--fetch` flag all existed to make a destructive full reload safe. There is no
destructive full reload.

**What survives is one thing, and it belongs to ticket 03:** the early-warning
signal for an upstream retitle silently orphaning chart text. That does not need
a vendored 4.7MB file to detect — see 03's assessment.

- [ ] `ingest` takes a path argument; upstream fetching moves behind an explicit
      `--fetch` flag or a separate binary
- [ ] Current upstream snapshot vendored (4.7MB) with its `Last-Modified` recorded
- [ ] Sanity checks before `TRUNCATE`: file parses, song count within a sane delta
      of the current DB, non-empty. Refuse and exit non-zero otherwise.
- [ ] Private data repo created; layout documented (snapshot + `maidata.txt` tree)
- [ ] `DATA_URL` constant removed or clearly marked as refresh-only
- [ ] CLAUDE.md / README updated: `cargo run --bin ingest` now needs a snapshot path
