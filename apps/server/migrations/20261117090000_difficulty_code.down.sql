-- The derived rows are left in place: the sync before this migration replaced
-- the table wholesale from upstream, so its first run removes them.
ALTER TABLE difficulties DROP COLUMN code;
