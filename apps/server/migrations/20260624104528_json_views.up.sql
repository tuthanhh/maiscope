-- JSON assembly views: emit the exact frontend shapes (camelCase keys) so the
-- handlers are one-liners and the response shape has a single source of truth.
-- Mirrors types/{Song,Sheet}.ts. The client still derives songNo/imageUrl/
-- sheetExpr/notePercents in preprocessData; those are not emitted here.

-- One JSON object per sheet, as nested under a song in data.json (sheet-only
-- fields + the Record<> maps).
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

-- The song-level fields shared by Song and (flattened) Sheet, as a jsonb object.
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

-- GET /songs/{id}: full Song = song meta + sheets[] (types/Song.ts).
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

-- GET /sheets/{sheetExpr}: standalone Sheet = song meta + sheet fields flattened
-- (types/Sheet.ts: Omit<Song,'sheets'> & {...}).
CREATE VIEW v_sheet AS
SELECT
    v.sheet_expr,
    (m.meta || v.doc::jsonb) AS doc
FROM v_sheet_obj v
JOIN v_song_meta m ON m.id = v.song_id_fk;
