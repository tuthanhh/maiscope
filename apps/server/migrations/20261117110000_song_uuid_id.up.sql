-- song-public-id 01: `songs.id` becomes a minted UUID v7, the identity URLs
-- will carry. `song_id` (upstream's text) stays as the sync's conflict key.
--
-- One transaction (sqlx wraps each migration), so `sheets.song_id_fk` is
-- never without a target. It is the only column referencing `songs(id)`;
-- `sheets`, `charts` and the `sheet_*` tables keep their bigserial keys.
--
-- `uuidv7()` is built in from Postgres 18, which dev, CI and Neon all run.
-- The DEFAULT is what mints for every later insert: the sync never names
-- `id`, so a new song gets one and an existing song keeps its own.

ALTER TABLE songs ADD COLUMN new_id UUID;
-- Minted once: sqlx records this migration as applied and never runs it
-- again, and after it the column default mints only for new rows. A down
-- migration followed by this one does mint fresh ids; nothing outside the
-- database holds one yet.
UPDATE songs SET new_id = uuidv7();
ALTER TABLE songs ALTER COLUMN new_id SET NOT NULL;

ALTER TABLE sheets ADD COLUMN new_song_id_fk UUID;
UPDATE sheets sh SET new_song_id_fk = s.new_id FROM songs s WHERE s.id = sh.song_id_fk;
ALTER TABLE sheets ALTER COLUMN new_song_id_fk SET NOT NULL;

-- Swap the key. Dropping the old columns takes their constraints, the
-- `sheets_song_idx` index and the `songs_id_seq` sequence with them.
ALTER TABLE sheets DROP COLUMN song_id_fk;
ALTER TABLE songs DROP COLUMN id;
ALTER TABLE songs RENAME COLUMN new_id TO id;
ALTER TABLE songs ALTER COLUMN id SET DEFAULT uuidv7();
ALTER TABLE songs ADD CONSTRAINT songs_pkey PRIMARY KEY (id);

ALTER TABLE sheets RENAME COLUMN new_song_id_fk TO song_id_fk;
ALTER TABLE sheets
    ADD CONSTRAINT sheets_song_id_fk_fkey
    FOREIGN KEY (song_id_fk) REFERENCES songs (id) ON DELETE CASCADE;
CREATE INDEX sheets_song_idx ON sheets (song_id_fk);
