CREATE VIEW v_sheet_obj AS
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

CREATE VIEW v_song_meta AS
SELECT
    so.id,
    so.song_id,
    so.source_index,
    jsonb_build_object(
        'songId',      so.song_id,
        'category',    so.category,
        'title',       so.title,
        'artist',      so.artist,
        'bpm',         so.bpm,
        'imageName',   so.image_name,
        'version',     so.version,
        'releaseDate', to_char(so.release_date, 'YYYY-MM-DD'),
        'isNew',       so.is_new,
        'isLocked',    so.is_locked,
        'comment',     so.comment
    ) AS meta
FROM songs so;

CREATE VIEW v_song AS
SELECT
    m.id,
    m.song_id,
    m.source_index,
    (m.meta || jsonb_build_object(
        'sheets', COALESCE((
            SELECT json_agg(v.doc ORDER BY v.source_index)
            FROM v_sheet_obj v WHERE v.song_id_fk = m.id
        ), '[]'::json)::jsonb
    )) AS doc
FROM v_song_meta m;

CREATE VIEW v_sheet AS
SELECT
    v.sheet_expr,
    (m.meta || v.doc::jsonb) AS doc
FROM v_sheet_obj v
JOIN v_song_meta m ON m.id = v.song_id_fk;
