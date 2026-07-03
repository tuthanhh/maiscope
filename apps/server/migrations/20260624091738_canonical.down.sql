-- Reverse of 20260624091738_canonical.up.sql.
-- CASCADE handles FK order; IF EXISTS keeps it safe to re-run.
DROP TABLE IF EXISTS sheet_region_overrides CASCADE;
DROP TABLE IF EXISTS sheet_regions          CASCADE;
DROP TABLE IF EXISTS sheet_note_counts       CASCADE;
DROP TABLE IF EXISTS sheets                  CASCADE;
DROP TABLE IF EXISTS songs                   CASCADE;
DROP TABLE IF EXISTS regions                 CASCADE;
DROP TABLE IF EXISTS difficulties            CASCADE;
DROP TABLE IF EXISTS types                   CASCADE;
DROP TABLE IF EXISTS versions                CASCADE;
DROP TABLE IF EXISTS categories              CASCADE;
DROP TABLE IF EXISTS catalog_meta            CASCADE;
