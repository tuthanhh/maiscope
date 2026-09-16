# 01 — The difficulties table covers UTAGE

**What to build:** every value appearing in `sheets.difficulty` has a row in
`difficulties`.

Upstream's `data.json` sends five difficulty rows and never mentions the 50
UTAGE labels, though its sheets use them. So the rows have to be derived from
the sheets themselves during the sync.

**Blocked by:** None.

**Status:** todo

- [ ] `catalog_sync` collects the distinct difficulty values from the sheets it
      is about to write, and ensures a `difficulties` row exists for each
- [ ] Synthesised rows survive the wholesale `DELETE FROM difficulties` +
      re-insert that the sync does today — reconstructed every run, not
      preserved by accident
- [ ] `ordinal` places UTAGE labels after the five standard ones, so the
      frontend's index maps keep their existing meaning for existing rows
- [ ] `name` for a synthesised row: decide whether it is the bare label, the
      bracketed form, or something readable. The catalog stores `【協】` on
      sheets while maidata emits `協`
- [ ] A test asserting no value in `sheets.difficulty` is absent from
      `difficulties` after a sync
- [ ] `docs/reference/schema.md` notes that `difficulties` is now partly derived
