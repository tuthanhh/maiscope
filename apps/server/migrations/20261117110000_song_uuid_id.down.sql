-- Back to a bigserial key. The original integers are gone, so new ones are
-- numbered in upstream order (`source_index`), and the sequence resumes
-- after the highest. Sheets follow their song; charts hang off sheets,
-- whose keys never changed.

ALTER TABLE songs ADD COLUMN old_id BIGINT;
UPDATE songs s SET old_id = r.n
FROM (SELECT id, row_number() OVER (ORDER BY source_index, id) AS n FROM songs) r
WHERE r.id = s.id;
ALTER TABLE songs ALTER COLUMN old_id SET NOT NULL;

ALTER TABLE sheets ADD COLUMN old_song_id_fk BIGINT;
UPDATE sheets sh SET old_song_id_fk = s.old_id FROM songs s WHERE s.id = sh.song_id_fk;
ALTER TABLE sheets ALTER COLUMN old_song_id_fk SET NOT NULL;

ALTER TABLE sheets DROP COLUMN song_id_fk;
ALTER TABLE songs DROP COLUMN id;
ALTER TABLE songs RENAME COLUMN old_id TO id;
CREATE SEQUENCE songs_id_seq OWNED BY songs.id;
SELECT setval('songs_id_seq', COALESCE((SELECT max(id) FROM songs), 0) + 1, false);
ALTER TABLE songs ALTER COLUMN id SET DEFAULT nextval('songs_id_seq');
ALTER TABLE songs ADD CONSTRAINT songs_pkey PRIMARY KEY (id);

ALTER TABLE sheets RENAME COLUMN old_song_id_fk TO song_id_fk;
ALTER TABLE sheets
    ADD CONSTRAINT sheets_song_id_fk_fkey
    FOREIGN KEY (song_id_fk) REFERENCES songs (id) ON DELETE CASCADE;
CREATE INDEX sheets_song_idx ON sheets (song_id_fk);
