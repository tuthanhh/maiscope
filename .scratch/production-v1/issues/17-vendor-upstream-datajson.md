# 17 — Vendor the upstream `data.json` snapshot

**What to build:** `bin/ingest.rs:17` hardcodes
`https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json` — a third party's bucket —
and `bin/ingest.rs:182` `TRUNCATE`s the canonical tables before reloading from it.
That combination means an upstream outage, a malformed file, or a truncated
response can wipe the catalog and reload garbage in one command.

Change `ingest` to read a **vendored snapshot file**. Refreshing upstream becomes
a reviewable commit (issue 18), so a rename or a shrink is visible in `git diff`
*before* it is applied — which is also the earliest place to catch the title
mismatches that make `seed_songs` drop charts (issue 10).

The snapshot lives in the private data repo alongside the chart text, so one
artifact serves as seed input and as backup.

**Blocked by:** None (independent of the server restructure)

**Status:** todo

- [ ] `ingest` takes a path argument; upstream fetching moves behind an explicit
      `--fetch` flag or a separate binary
- [ ] Current upstream snapshot vendored (4.7MB) with its `Last-Modified` recorded
- [ ] Sanity checks before `TRUNCATE`: file parses, song count within a sane delta
      of the current DB, non-empty. Refuse and exit non-zero otherwise.
- [ ] Private data repo created; layout documented (snapshot + `maidata.txt` tree)
- [ ] `DATA_URL` constant removed or clearly marked as refresh-only
- [ ] CLAUDE.md / README updated: `cargo run --bin ingest` now needs a snapshot path
