-- api-rewrite: one sheet per (song, type, difficulty), the address the v2
-- routes look sheets up by. Already true by construction — `sheet_expr` is
-- UNIQUE and is built from the song's upstream key, the type and the
-- difficulty — but the lookups take the first row they find, so the guarantee
-- belongs in the schema. As an index it also serves those lookups.
ALTER TABLE sheets
    ADD CONSTRAINT sheets_song_type_difficulty_key UNIQUE (song_id_fk, type, difficulty);
