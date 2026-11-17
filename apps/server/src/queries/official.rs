//! Songs and sheets of the official library, in their v2 shape
//! (`api-rewrite` 03, 04). Two queries with optional filters serve the
//! Catalog (no filter), one song (by id) and one sheet (by id, type and
//! difficulty code), so the three payloads cannot drift apart.
//!
//! The `$n IS NULL OR …` filters cost the single-song paths their index once
//! Postgres settles on a generic plan: a scan of ~7.5k sheets, milliseconds.
//! Accepted for one query shape over three; split them if it ever shows.
//!
//! Rows upstream no longer lists are hidden: a song or sheet with a standing
//! tombstone in `deleted_songs` / `deleted_sheets` is not served (ADR-0026).
//! The sync clears the tombstone when the row returns. A vanished song's
//! sheets vanish with it, so the sheet filter alone covers the sheet routes.

use sqlx::PgConnection;
use sqlx::types::Uuid;

use crate::types::{OfficialSheetRow, OfficialSongRow};

type NoteCountsJson = sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>;
type RegionsJson = sqlx::types::Json<std::collections::BTreeMap<String, bool>>;
type RegionOverridesJson =
    sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>;

/// Every song, or the one with `id`, in `source_index` order.
pub async fn fetch_official_songs(
    conn: &mut PgConnection,
    id: Option<Uuid>,
) -> Result<Vec<OfficialSongRow>, sqlx::Error> {
    sqlx::query_as!(
        OfficialSongRow,
        r#"SELECT id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_locked, comment
           FROM songs
           WHERE ($1::uuid IS NULL OR id = $1)
             AND NOT EXISTS (SELECT 1 FROM deleted_songs d WHERE d.song_id = songs.song_id)
           ORDER BY source_index, id"#,
        id
    )
    .fetch_all(conn)
    .await
}

/// Sheets, narrowed by song and then by type and difficulty code, each `None`
/// meaning "any". In song then `source_index` order. The difficulty leaves as
/// `difficulties.code`, the form URLs carry, not the stored key.
pub async fn fetch_official_sheets(
    conn: &mut PgConnection,
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
                SELECT 1 FROM charts c
                WHERE c.sheet_id = s.id AND c.format = 'maimai-simai' AND c.content IS NOT NULL
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
             AND NOT EXISTS (SELECT 1 FROM deleted_sheets t WHERE t.sheet_expr = s.sheet_expr)
           ORDER BY s.song_id_fk, s.source_index"#,
        song_id,
        r#type,
        difficulty_code
    )
    .fetch_all(conn)
    .await
}

/// What the chart endpoint can say about one sheet.
pub enum ChartText {
    /// No such sheet.
    NoSheet,
    /// The sheet exists; no chart row does.
    NoChart,
    /// A chart row with no inline text (blob-only, not served yet).
    NotInline,
    Inline(String),
}

/// The simai text for one sheet, addressed as the v2 routes address it. One
/// statement, so the four outcomes come from one snapshot.
pub async fn fetch_chart_text(
    conn: &mut PgConnection,
    song_id: Uuid,
    r#type: &str,
    difficulty_code: &str,
) -> Result<ChartText, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT c.id IS NOT NULL AS "has_row!", c.content
           FROM sheets s
           JOIN difficulties d ON d.difficulty = s.difficulty
           LEFT JOIN charts c ON c.sheet_id = s.id AND c.format = 'maimai-simai'
           WHERE s.song_id_fk = $1 AND s.type = $2 AND d.code = $3
             AND NOT EXISTS (SELECT 1 FROM deleted_sheets t WHERE t.sheet_expr = s.sheet_expr)"#,
        song_id,
        r#type,
        difficulty_code
    )
    .fetch_optional(conn)
    .await?;
    Ok(match row {
        None => ChartText::NoSheet,
        Some(row) if !row.has_row => ChartText::NoChart,
        Some(row) => match row.content {
            Some(text) => ChartText::Inline(text),
            None => ChartText::NotInline,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two songs and three sheets, each inserted out of `source_index` order,
    /// so a missing `ORDER BY` shows. Returns (first song, second song).
    async fn seed(pool: &sqlx::PgPool) -> sqlx::Result<(Uuid, Uuid)> {
        crate::test_support::seed_difficulties(pool, &["master", "expert"]).await;
        sqlx::query!(
            "INSERT INTO difficulties (difficulty, name, code, ordinal)
             VALUES ('【協】', '【協】', '協', 9)"
        )
        .execute(pool)
        .await?;
        let second = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ('b', 'B', 1) RETURNING id"
        )
        .fetch_one(pool)
        .await?;
        let first = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ('a', 'A', 0) RETURNING id"
        )
        .fetch_one(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, 'a|utage|【協】', 'utage', '【協】', 2),
                    ($1, 'a|dx|master', 'dx', 'master', 1),
                    ($1, 'a|dx|expert', 'dx', 'expert', 0),
                    ($2, 'b|dx|master', 'dx', 'master', 0)",
            first,
            second
        )
        .execute(pool)
        .await?;
        Ok((first, second))
    }

    fn keys(rows: &[OfficialSheetRow]) -> Vec<(String, String)> {
        rows.iter()
            .map(|r| (r.r#type.clone().unwrap(), r.difficulty.clone().unwrap()))
            .collect()
    }

    #[sqlx::test]
    async fn songs_come_in_source_order_and_filter_by_id(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let (first, second) = seed(&pool).await?;
        let mut conn = pool.acquire().await?;
        let all = fetch_official_songs(&mut conn, None).await?;
        assert_eq!(
            all.iter().map(|s| s.id).collect::<Vec<_>>(),
            [first, second]
        );
        let one = fetch_official_songs(&mut conn, Some(second)).await?;
        assert_eq!(one.iter().map(|s| s.id).collect::<Vec<_>>(), [second]);
        Ok(())
    }

    #[sqlx::test]
    async fn tombstoned_rows_are_hidden(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let (first, second) = seed(&pool).await?;
        sqlx::query!("INSERT INTO deleted_songs (song_id, revision) VALUES ('b', 1)")
            .execute(&pool)
            .await?;
        sqlx::query!(
            "INSERT INTO deleted_sheets (sheet_expr, revision)
             VALUES ('b|dx|master', 1), ('a|dx|expert', 1)"
        )
        .execute(&pool)
        .await?;
        let mut conn = pool.acquire().await?;

        let songs = fetch_official_songs(&mut conn, None).await?;
        assert_eq!(songs.iter().map(|s| s.id).collect::<Vec<_>>(), [first]);
        assert!(
            fetch_official_songs(&mut conn, Some(second))
                .await?
                .is_empty()
        );

        let pair = |t: &str, d: &str| (t.to_string(), d.to_string());
        let sheets = fetch_official_sheets(&mut conn, None, None).await?;
        assert_eq!(keys(&sheets), [pair("dx", "master"), pair("utage", "協")]);
        assert!(matches!(
            fetch_chart_text(&mut conn, first, "dx", "expert").await?,
            ChartText::NoSheet
        ));
        Ok(())
    }

    #[sqlx::test]
    async fn sheets_filter_by_song_then_by_type_and_code(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let (first, second) = seed(&pool).await?;
        let mut conn = pool.acquire().await?;
        let pair = |t: &str, d: &str| (t.to_string(), d.to_string());

        let all = fetch_official_sheets(&mut conn, None, None).await?;
        assert_eq!(all.len(), 4);

        // Per song, in `source_index` order, UTAGE as its code.
        let mine = fetch_official_sheets(&mut conn, Some(first), None).await?;
        assert_eq!(
            keys(&mine),
            [
                pair("dx", "expert"),
                pair("dx", "master"),
                pair("utage", "協")
            ]
        );

        let one = fetch_official_sheets(&mut conn, Some(first), Some(("utage", "協"))).await?;
        assert_eq!(keys(&one), [pair("utage", "協")]);
        // The stored key is not an address; neither is another song's sheet.
        assert!(
            fetch_official_sheets(&mut conn, Some(first), Some(("utage", "【協】")))
                .await?
                .is_empty()
        );
        let theirs = fetch_official_sheets(&mut conn, Some(second), Some(("dx", "expert"))).await?;
        assert!(theirs.is_empty());
        Ok(())
    }
}
