DROP TABLE IF EXISTS deleted_sheets;
DROP TABLE IF EXISTS deleted_songs;
ALTER TABLE sheets DROP COLUMN revision;
ALTER TABLE songs DROP COLUMN revision;
ALTER TABLE catalog_meta DROP COLUMN last_full_reload_revision;
ALTER TABLE catalog_meta DROP COLUMN revision;
