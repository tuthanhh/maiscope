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
- [ ] **Two columns, not one.** `name` is the display form (`【協】`, matching
      what `sheets` already stores); `code` is the matching and URL form (`協`,
      matching what maidata emits). The five standard rows get a `code` too
      (`basic`, `master`, …), which they effectively already have
- [ ] A `UNIQUE` constraint on `code` — it becomes a URL component
- [ ] A test asserting no value in `sheets.difficulty` is absent from
      `difficulties` after a sync
- [ ] A test asserting `code` is `name` with the brackets stripped, for every
      synthesised row — the two columns must not drift
- [ ] `docs/reference/schema.md` notes that `difficulties` is now partly derived,
      and documents the `name`/`code` split

## Comments

**2026-09-16 — why two columns rather than picking one.**

Measured against dev Postgres: 176 UTAGE sheets, **50 distinct labels**, and all
176 match `^【.】$` exactly — always full-width brackets, always one character
inside. Some of those characters are ASCII (`【X】`, `【J】`, `【r】`), and `r` is
lowercase, so whichever form is chosen is case-sensitive.

Storing one form and deriving the other means a strip-or-add transformation
applied in every direction: parser output, URL parsing, display, and community
upload validation. That is the class of bug that produces "the chart is there
but nothing finds it". A column is cheaper than a convention, and `difficulties`
is becoming derived infrastructure rather than a dumb mirror anyway.

The URL component is `code` — see [`api-rewrite`](../../api-rewrite/spec.md),
which asked the same question separately and must not get a different answer.

