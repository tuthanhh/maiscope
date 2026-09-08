-- Sync tier (contract §3): a monotonic revision counter so clients can ask
-- "what changed since revision N" instead of refetching the whole catalog.

ALTER TABLE catalog_meta
    ADD COLUMN revision BIGINT NOT NULL DEFAULT 0,
    -- The revision at the last full TRUNCATE+reload (see bin/ingest.rs).
    -- GET /sync/delta can only diff incrementally *after* this point — a full
    -- reload has no stable row identity to diff against the client's cache.
    ADD COLUMN last_full_reload_revision BIGINT NOT NULL DEFAULT 0;

ALTER TABLE songs  ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;
ALTER TABLE sheets ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;

CREATE INDEX songs_revision_idx  ON songs  (revision);
CREATE INDEX sheets_revision_idx ON sheets (revision);

-- Deletion log. Nothing in this codebase can delete a canonical song/sheet
-- yet (no such endpoint exists in the contract) — these tables exist so
-- GET /sync/delta's response shape is complete now, populated once a
-- deletion capability is added later.
CREATE TABLE deleted_songs (
    song_id    TEXT   NOT NULL,
    revision   BIGINT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (song_id, revision)
);
CREATE TABLE deleted_sheets (
    sheet_expr TEXT   NOT NULL,
    revision   BIGINT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (sheet_expr, revision)
);

CREATE INDEX deleted_songs_revision_idx ON deleted_songs (revision);
CREATE INDEX deleted_sheets_revision_idx ON deleted_sheets (revision);
