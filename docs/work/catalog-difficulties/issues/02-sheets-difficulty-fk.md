# 02 — `sheets.difficulty` references `difficulties`

**What to build:** a foreign key, so a sheet cannot hold a difficulty the table
does not know.

This is what turns the API rewrite's `Difficulty::Utage(String)` from a string
that is merely shaped right into a value the database has checked, and what lets
community uploads be validated by a constraint rather than by a reviewer.

**Blocked by:** 01 — the table must already cover every value in use, or adding
the constraint fails on existing rows.

**Status:** todo

- [ ] Migration adds `FOREIGN KEY (difficulty) REFERENCES difficulties(difficulty)`
- [ ] Ordering inside `catalog_sync` guarantees difficulty rows are written
      before the sheets referencing them — a batch that inserts sheets first
      will now fail rather than silently succeed
- [ ] `ON DELETE` behaviour chosen deliberately: a difficulty should not be
      removable while sheets use it
- [ ] `.sqlx/` regenerated
- [ ] `schema.md` updated
