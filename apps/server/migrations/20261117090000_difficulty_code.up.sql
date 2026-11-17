-- catalog-difficulties 01: `difficulties` covers every value `sheets.difficulty`
-- holds, not just the five upstream lists.
--
-- `name` stays the display form; for a derived UTAGE row that is the bracketed
-- tag the sheets store (`【協】`). `code` is the matching and URL form: the bare
-- tag maidata emits (`協`), or the difficulty itself for the standard rows.
-- Two columns rather than one plus a strip-or-add rule on every path.
ALTER TABLE difficulties ADD COLUMN code TEXT;
UPDATE difficulties SET code = difficulty;

-- Backfill the derived rows now rather than waiting for the next sync: the
-- foreign key that follows this migration fails on any sheet whose difficulty
-- has no row, and a deploy runs both migrations before any sync can.
--
-- Same rule as `catalog_sync::difficulty_rows`, so the next sync rebuilds these
-- rows byte-identically and does not advance the catalog revision: after the
-- existing rows, ordered by byte value (COLLATE "C" is Rust's `String` order).
INSERT INTO difficulties (difficulty, name, code, ordinal)
SELECT d.difficulty,
       d.difficulty,
       CASE WHEN d.difficulty ~ '^【.+】$'
            THEN substr(d.difficulty, 2, char_length(d.difficulty) - 2)
            ELSE d.difficulty END,
       (SELECT COALESCE(max(ordinal), -1) FROM difficulties)
         + row_number() OVER (ORDER BY d.difficulty COLLATE "C")
FROM (SELECT DISTINCT difficulty FROM sheets
      WHERE difficulty IS NOT NULL
        AND difficulty NOT IN (SELECT difficulty FROM difficulties)) d;

ALTER TABLE difficulties ALTER COLUMN code SET NOT NULL;
-- `code` becomes a URL component.
ALTER TABLE difficulties ADD CONSTRAINT difficulties_code_key UNIQUE (code);
