# 02 — `sheets.difficulty` references `difficulties`

**What to build:** a foreign key, so a sheet cannot hold a difficulty the table
does not know.

This is what turns the API rewrite's `Difficulty::Utage(String)` from a string
that is merely shaped right into a value the database has checked, and what lets
community uploads be validated by a constraint rather than by a reviewer.

**Blocked by:** 01 — the table must already cover every value in use, or adding
the constraint fails on existing rows.

**Status:** done (2026-11-17)

- [x] Migration adds `FOREIGN KEY (difficulty) REFERENCES difficulties(difficulty)`
- [x] Ordering inside `catalog_sync` guarantees difficulty rows are written
      before the sheets referencing them — a batch that inserts sheets first
      will now fail rather than silently succeed
- [x] `ON DELETE` behaviour chosen deliberately: a difficulty should not be
      removable while sheets use it
- [x] `.sqlx/` regenerated
- [x] `schema.md` updated

## Comments

**2026-11-17 — done.** `ON DELETE RESTRICT`, checked at the statement
rather than deferred to commit, so a violation names the statement that
caused it. The key ruled out the sync's wholesale `DELETE FROM
difficulties`, so that table alone is now upserted and pruned of rows no
sheet uses. The derived set already contains every stored value, so the
prune never hits the restriction. An upstream row dropped while sheets use
it turns into a derived row instead of failing the sync. That case has a
test.

Every test fixture that inserts a sheet with a difficulty now seeds the
row first. That is the key working, not test noise. On dev, a sync after
both migrations changed nothing and left the table's md5 unchanged.
