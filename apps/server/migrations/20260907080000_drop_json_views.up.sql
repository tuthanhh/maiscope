-- The handlers no longer read from these JSON-assembly views (see
-- src/queries.rs) — the response shape's single source of truth is now the
-- typed structs in src/types.rs, queried directly against the base tables.
DROP VIEW v_song;
DROP VIEW v_sheet;
DROP VIEW v_sheet_obj;
DROP VIEW v_song_meta;
