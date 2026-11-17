-- catalog-difficulties 02: a sheet cannot hold a difficulty `difficulties`
-- does not know. The previous migration backfilled a row for every value in
-- use, so this applies to existing data.
--
-- ON DELETE RESTRICT: a difficulty is not removable while any sheet uses it,
-- checked at the statement rather than deferred. `bin/sync_catalog` therefore
-- upserts this table and prunes only unused rows instead of replacing it.
-- No ON UPDATE action: the sync never rewrites a `difficulty` key.
ALTER TABLE sheets
    ADD CONSTRAINT sheets_difficulty_fkey
    FOREIGN KEY (difficulty) REFERENCES difficulties (difficulty)
    ON DELETE RESTRICT;
