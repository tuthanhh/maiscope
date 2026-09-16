# 01 — Mint and backfill `public_id`

**What to build:** a `songs.public_id` column, populated for all 1845 existing
rows.

**Blocked by:** None.

**Status:** todo

- [ ] The format is **UUID v7 in a native `uuid` column**, rendered canonical
      hyphenated in URLs. Add the `uuid` crate with its `v7` and `serde`
      features; sqlx already builds with its `uuid` feature
      (`apps/server/Cargo.toml:12`)
- [ ] Migration adds the column, backfills every row, then applies
      `UNIQUE NOT NULL` — in that order, since the constraint cannot precede the
      backfill
- [ ] Index on `public_id`, since it becomes the API's primary lookup
- [ ] Backfill is idempotent: re-running must not re-mint an existing id, or
      every URL in existence changes
- [ ] A down migration exists and is tested — this is the one change here that
      is painful to reverse by hand
- [ ] `.sqlx/` regenerated; `schema.md` documents the column and why it is not
      `song_id`

## Comments

**2026-09-16 — why UUID v7 rather than the short random string this ticket
originally suggested.**

v7 over v4: the embedded millisecond timestamp publishes only the row's creation
time, which is sync time for a song and upload time for a community song — both
already displayed. It is not guessable (74 random bits), and it keeps the index
compact, though at 1845 rows that part is irrelevant.

Native `uuid` over `TEXT`: 16 bytes rather than 37, validated by Postgres, and a
distinct Rust type — so passing a `song_id` where a `public_id` belongs becomes
a compile error instead of a silent lookup miss, which is the exact class of
mistake this feature exists to end.

A 10–12 character base32 id would give 3× shorter URLs with the same
non-guessability, and was rejected: it needs a generator, a format `CHECK` and
collision handling that this project would own and test itself. URL length is an
aesthetic cost paid once by a string nobody types. The same scheme covers
`community_songs.public_id` and `community_charts.public_id` — one addressing
scheme across both libraries.

