-- Chart storage: the actual simai/ma2 data feeding the wasm engine
-- (composables/useEngine.ts). Canonical chart per (sheet, format); history in
-- chart_revisions for rollback/audit. Contributor/contribution FKs are added in
-- the contributions migration (Phase 3) — kept as plain nullable ids for now.

CREATE TABLE charts (
    id                  BIGSERIAL   PRIMARY KEY,
    sheet_id            BIGINT      NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
    sheet_expr          TEXT        NOT NULL,            -- denormalized cross-tier key
    format              TEXT        NOT NULL DEFAULT 'maimai-simai',
    content             TEXT,                            -- inline simai/ma2 text
    blob_url            TEXT,                            -- or S3 key for large/binary
    hash                TEXT,
    contributor_user_id BIGINT,                          -- FK added in Phase 3
    approved_at         TIMESTAMPTZ,
    version             INTEGER     NOT NULL DEFAULT 1,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (sheet_id, format)                            -- one canonical chart per format
);

CREATE INDEX charts_sheet_expr_idx ON charts (sheet_expr);

-- Append-only history. Each approved contribution adds a row; enables rollback.
CREATE TABLE chart_revisions (
    id              BIGSERIAL   PRIMARY KEY,
    chart_id        BIGINT      NOT NULL REFERENCES charts(id) ON DELETE CASCADE,
    contribution_id BIGINT,                              -- FK added in Phase 3
    content         TEXT,
    blob_url        TEXT,
    hash            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX chart_revisions_chart_idx ON chart_revisions (chart_id);

-- Expose hasChart on each sheet so the catalog/UI can show a "visualize"
-- affordance without a probe (contract §1.1). Column list unchanged → REPLACE is
-- safe and leaves dependent views (v_song, v_sheet) intact.
CREATE OR REPLACE VIEW v_sheet_obj AS
SELECT
    s.id,
    s.song_id_fk,
    s.sheet_expr,
    s.source_index,
    json_build_object(
        'type',               s.type,
        'difficulty',         s.difficulty,
        'level',              s.level,
        'levelValue',         s.level_value,
        'internalLevel',      s.internal_level,
        'internalLevelValue', s.internal_level_value,
        'noteDesigner',       s.note_designer,
        'isSpecial',          s.is_special,
        'hasChart', EXISTS (
            SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
        ),
        'noteCounts', (
            SELECT json_object_agg(key, value)
            FROM sheet_note_counts WHERE sheet_id = s.id
        ),
        'regions', (
            SELECT json_object_agg(region, available)
            FROM sheet_regions WHERE sheet_id = s.id
        ),
        'regionOverrides', (
            SELECT json_object_agg(region, json_build_object(
                'level',              level,
                'levelValue',         level_value,
                'internalLevel',      internal_level,
                'internalLevelValue', internal_level_value,
                'noteDesigner',       note_designer
            ))
            FROM sheet_region_overrides WHERE sheet_id = s.id
        )
    ) AS doc
FROM sheets s;
