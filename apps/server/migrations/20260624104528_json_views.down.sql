-- Reverse of 20260624104528_json_views.up.sql.
-- Drop in dependency order (v_song/v_sheet build on the bases); IF EXISTS keeps
-- it idempotent. Views hold no data, so no data loss.
DROP VIEW IF EXISTS v_sheet;
DROP VIEW IF EXISTS v_song;
DROP VIEW IF EXISTS v_song_meta;
DROP VIEW IF EXISTS v_sheet_obj;
