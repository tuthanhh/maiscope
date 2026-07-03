-- Reverse of 20260624105451_charts.up.sql.
-- Restore v_sheet_obj to its 0002 form FIRST (drops the charts reference), then
-- drop the chart tables. IF EXISTS keeps it idempotent.
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

DROP TABLE IF EXISTS chart_revisions CASCADE;
DROP TABLE IF EXISTS charts CASCADE;
