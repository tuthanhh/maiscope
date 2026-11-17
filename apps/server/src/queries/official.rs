//! Songs and sheets of the official library, in their v2 shape
//! (`api-rewrite` 03, 04). Two queries with optional filters serve the
//! Catalog (no filter), one song (by id) and one sheet (by id, type and
//! difficulty code), so the three payloads cannot drift apart.

use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::types::{OfficialSheetRow, OfficialSongRow};

type NoteCountsJson = sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>;
type RegionsJson = sqlx::types::Json<std::collections::BTreeMap<String, bool>>;
type RegionOverridesJson =
    sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>;

/// Every song, or the one with `id`, in `source_index` order.
pub async fn fetch_official_songs(
    pool: &PgPool,
    id: Option<Uuid>,
) -> Result<Vec<OfficialSongRow>, sqlx::Error> {
    sqlx::query_as!(
        OfficialSongRow,
        r#"SELECT id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_locked, comment
           FROM songs
           WHERE $1::uuid IS NULL OR id = $1
           ORDER BY source_index"#,
        id
    )
    .fetch_all(pool)
    .await
}

/// Sheets, narrowed by song and then by type and difficulty code, each `None`
/// meaning "any". In song then `source_index` order. The difficulty leaves as
/// `difficulties.code`, the form URLs carry, not the stored key.
pub async fn fetch_official_sheets(
    pool: &PgPool,
    song_id: Option<Uuid>,
    sheet: Option<(&str, &str)>,
) -> Result<Vec<OfficialSheetRow>, sqlx::Error> {
    let (r#type, difficulty_code) = sheet.unzip();
    sqlx::query_as!(
        OfficialSheetRow,
        r#"SELECT
            s.song_id_fk AS song_id,
            s.type,
            d.code AS "difficulty?",
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            EXISTS (
                SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
            ) AS "has_chart!",
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id)
                AS "note_counts: NoteCountsJson",
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id)
                AS "regions: RegionsJson",
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id)
                AS "region_overrides: RegionOverridesJson"
           FROM sheets s
           LEFT JOIN difficulties d ON d.difficulty = s.difficulty
           WHERE ($1::uuid IS NULL OR s.song_id_fk = $1)
             AND ($2::text IS NULL OR (s.type = $2 AND d.code = $3))
           ORDER BY s.song_id_fk, s.source_index"#,
        song_id,
        r#type,
        difficulty_code
    )
    .fetch_all(pool)
    .await
}
