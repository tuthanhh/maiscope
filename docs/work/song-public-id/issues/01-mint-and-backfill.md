# 01 — `songs.id` becomes a UUID

**What to build:** replace the bigserial primary key on `songs` with a minted
UUID, for all 1845 existing rows, carrying `sheets.song_id_fk` with it.

There is no separate `public_id` column: the primary key *is* the public
identifier. See the spec for the reversal and what it costs.

**Blocked by:** None.

**Status:** done (2026-11-17)

- [x] The format is **UUID v7 in a native `uuid` column**, rendered canonical
      hyphenated in URLs
- [ ] ~~Add the `uuid` crate with its `v7` and `serde` features~~ deferred to
      `api-rewrite`, the first code to mint or serialize an id in Rust; sqlx's
      re-exported `Uuid` covers reading and binding (see the comment below)
- [x] Migration mints a UUID per existing song, rewrites `sheets.song_id_fk` to
      match, then swaps the primary key and drops the bigserial — one
      transaction, so there is no window in which the FK points at nothing.
      `sheets.song_id_fk` is the **only** column referencing `songs(id)`
      (`20260624091738_canonical.up.sql:90`); `sheets`, `charts` and the
      `sheet_*` tables keep their bigserial keys
- [x] Every `sqlx::query!` call site binding a song id changes type — `.sqlx/`
      must be regenerated and `cargo build` is the check that none were missed
- [x] Backfill is idempotent: re-running must not re-mint an existing id, or
      every URL in existence changes
- [x] A down migration exists and is tested — this is the one change here that
      is painful to reverse by hand, and now it moves a primary key
- [x] `.sqlx/` regenerated; `schema.md` documents the key and why it is not
      `song_id`
- [x] A test asserting a re-insert path reuses an existing song's id by matching
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

**2026-11-17: done.** Minted in Postgres, not Rust: the column default is
`uuidv7()`, built in from Postgres 18. Neon already ran 18; dev and CI
moved up from 17 in a commit of their own, which also closes that version
gap. The `uuid` crate was **not** added: the server only reads and binds
ids, and sqlx's re-exported `Uuid` covers that. It comes in when something
mints or serializes an id in Rust, which is `api-rewrite`'s business.

The default is what makes ticket 02 mostly free: the sync never names `id`,
so a new song is minted one and an existing song keeps its own. The
re-insert test covers that path here.

Measured on dev (1879 songs, 7481 sheets, 7065 charts): every song got a
distinct v7, and the song, sheet and chart links hashed identically before
and after the migration. The down migration renumbers songs in upstream
order, since the old integers are gone; a test runs it down and up again
with sheets and charts still attached. No `/api/v1` response changes: song
keys never reached a payload, and the catalog snapshot test still passes.

**2026-11-17: how it deploys.** The migration drops the old key columns
inside the release command, against the expand-and-contract rule. It
ships as a recorded exception with a backup-first runbook:
[ADR-0024](../../../adr/0024-song-id-migration-is-a-lossless-drop-on-deploy.md).
Minting in the database and the Postgres 18 floor:
[ADR-0023](../../../adr/0023-postgres-18-and-database-minted-song-ids.md).
