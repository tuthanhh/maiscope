# 01 — `songs.id` becomes a UUID

**What to build:** replace the bigserial primary key on `songs` with a minted
UUID, for all 1845 existing rows, carrying `sheets.song_id_fk` with it.

There is no separate `public_id` column: the primary key *is* the public
identifier. See the spec for the reversal and what it costs.

**Blocked by:** None.

**Status:** todo

- [ ] The format is **UUID v7 in a native `uuid` column**, rendered canonical
      hyphenated in URLs. Add the `uuid` crate with its `v7` and `serde`
      features; sqlx already builds with its `uuid` feature
      (`apps/server/Cargo.toml:12`)
- [ ] Migration mints a UUID per existing song, rewrites `sheets.song_id_fk` to
      match, then swaps the primary key and drops the bigserial — one
      transaction, so there is no window in which the FK points at nothing.
      `sheets.song_id_fk` is the **only** column referencing `songs(id)`
      (`20260624091738_canonical.up.sql:90`); `sheets`, `charts` and the
      `sheet_*` tables keep their bigserial keys
- [ ] Every `sqlx::query!` call site binding a song id changes type — `.sqlx/`
      must be regenerated and `cargo build` is the check that none were missed
- [ ] Backfill is idempotent: re-running must not re-mint an existing id, or
      every URL in existence changes
- [ ] A down migration exists and is tested — this is the one change here that
      is painful to reverse by hand, and now it moves a primary key
- [ ] `.sqlx/` regenerated; `schema.md` documents the key and why it is not
      `song_id`
- [ ] A test asserting a re-insert path reuses an existing song's id by matching
      on `song_id`. The primary key is identity-bearing now, so "re-inserting a
      song mints a new id" is no longer a harmless storage detail — it is every
      URL for that song changing

## Comments

**2026-09-16 — why UUID v7, and why it replaces the primary key rather than
sitting beside it.**

v7 over v4: the embedded millisecond timestamp publishes only the row's creation
time, which is sync time for a song and upload time for a community song — both
already displayed. It is not guessable (74 random bits), and it keeps the index
compact, though at 1845 rows that part is irrelevant.

Native `uuid` over `TEXT`: 16 bytes rather than 37, validated by Postgres, and a
distinct Rust type — so passing a `song_id` where a song `id` belongs becomes
a compile error instead of a silent lookup miss, which is the exact class of
mistake this feature exists to end.

A 10–12 character base32 id would give 3× shorter URLs with the same
non-guessability, and was rejected: it needs a generator, a format `CHECK` and
collision handling that this project would own and test itself. URL length is an
aesthetic cost paid once by a string nobody types. The same scheme covers `community_songs` and
`community_charts`, also as their primary keys — one addressing scheme across
both libraries, and one id per row rather than two.

Keeping a bigserial primary key beside a separate `public_id` was the earlier
plan and was reversed on the grounds that two ids for one row is two things to
keep in step, two things to pick the wrong one of in a query, and a permanent
question of which one a given function takes. The cost is real and named in the
spec: the primary key is now identity-bearing, so a re-insert must reuse the
existing id rather than mint a fresh one.

