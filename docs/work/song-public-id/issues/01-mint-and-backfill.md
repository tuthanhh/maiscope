# 01 — Mint and backfill `public_id`

**What to build:** a `songs.public_id` column, populated for all 1845 existing
rows.

**Blocked by:** None.

**Status:** todo

- [ ] Decide the format before writing the migration. It must be ASCII,
      URL-safe without encoding, stable, and not derived from any mutable field.
      A short random string is the obvious candidate; a sequence exposes row
      order and count, which is information the API need not leak
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
