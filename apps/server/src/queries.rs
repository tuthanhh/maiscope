use crate::types::{CategoryEntry, DifficultyEntry, RegionEntry, TypeEntry, VersionEntry};
use sqlx::PgPool;
use std::collections::HashMap;

pub async fn fetch_all_songs(pool: &PgPool) -> Result<Vec<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs ORDER BY source_index"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_song_by_song_id(
    pool: &PgPool,
    song_id: &str,
) -> Result<Option<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs WHERE song_id = $1"#,
        song_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_sheet_by_expr(
    pool: &PgPool,
    sheet_expr: &str,
) -> Result<Option<(crate::types::SongRow, crate::types::SheetRow)>, sqlx::Error> {
    let Some(song_pk) = sqlx::query_scalar!(
        "SELECT song_id_fk FROM sheets WHERE sheet_expr = $1",
        sheet_expr
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let song = sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs WHERE id = $1"#,
        song_pk
    )
    .fetch_one(pool)
    .await?;

    let sheet = sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
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
           WHERE s.sheet_expr = $1"#,
        sheet_expr
    )
    .fetch_one(pool)
    .await?;

    Ok(Some((song, sheet)))
}

type NoteCountsJson = sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>;
type RegionsJson = sqlx::types::Json<std::collections::BTreeMap<String, bool>>;
type RegionOverridesJson = sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>;

pub async fn fetch_all_sheets(
    pool: &PgPool,
    region: Option<&str>,
) -> Result<HashMap<i64, Vec<crate::types::SheetRow>>, sqlx::Error> {
    let rows = sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
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
           WHERE $1::text IS NULL
              OR EXISTS (
                  SELECT 1 FROM sheet_regions sr
                  WHERE sr.sheet_id = s.id AND sr.region = $1 AND sr.available
              )
           ORDER BY s.song_id_fk, s.source_index"#,
        region
    )
    .fetch_all(pool)
    .await?;

    let mut grouped: HashMap<i64, Vec<crate::types::SheetRow>> = HashMap::new();
    for row in rows {
        grouped.entry(row.song_id_fk).or_default().push(row);
    }
    Ok(grouped)
}

pub async fn fetch_sheets_for_song(
    pool: &PgPool,
    song_pk: i64,
) -> Result<Vec<crate::types::SheetRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
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
           WHERE s.song_id_fk = $1
           ORDER BY s.source_index"#,
        song_pk
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_categories(pool: &PgPool) -> Result<Vec<CategoryEntry>, sqlx::Error> {
    sqlx::query_as!(
        CategoryEntry,
        r#"SELECT category FROM categories ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_versions(pool: &PgPool) -> Result<Vec<VersionEntry>, sqlx::Error> {
    sqlx::query_as!(
        VersionEntry,
        r#"SELECT version, abbr, to_char(release_date, 'YYYY-MM-DD') AS release_date
           FROM versions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_types(pool: &PgPool) -> Result<Vec<TypeEntry>, sqlx::Error> {
    sqlx::query_as!(
        TypeEntry,
        r#"SELECT type AS "type: String", name, abbr, icon_url, icon_height
           FROM types ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_difficulties(pool: &PgPool) -> Result<Vec<DifficultyEntry>, sqlx::Error> {
    sqlx::query_as!(
        DifficultyEntry,
        r#"SELECT difficulty, name, color, icon_url, icon_height
           FROM difficulties ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_regions(pool: &PgPool) -> Result<Vec<RegionEntry>, sqlx::Error> {
    sqlx::query_as!(
        RegionEntry,
        r#"SELECT region, name FROM regions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn fetch_categories_orders_by_ordinal(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "maimai", 1)
            .execute(&pool)
            .await?;
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "pops", 0)
            .execute(&pool)
            .await?;

        let result = fetch_categories(&pool).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].category, "pops");
        assert_eq!(result[1].category, "maimai");
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_versions_formats_release_date(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal) VALUES ($1, $2, $3, $4)",
            "maimai DX",
            Some("DX"),
            Some(sqlx::types::chrono::NaiveDate::from_ymd_opt(2019, 7, 11).unwrap()),
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_versions(&pool).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].version, "maimai DX");
        assert_eq!(result[0].abbr.as_deref(), Some("DX"));
        assert_eq!(result[0].release_date.as_deref(), Some("2019-07-11"));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_update_time_reads_catalog_meta(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO catalog_meta (id, update_time) VALUES (true, $1)",
            sqlx::types::chrono::DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z")
                .unwrap()
                .with_timezone(&sqlx::types::chrono::Utc)
        )
        .execute(&pool)
        .await?;

        let result = fetch_update_time(&pool).await.unwrap();

        assert_eq!(result, "2026-09-01");
        Ok(())
    }

    async fn seed_song_with_two_sheets(pool: &PgPool) -> (i64, i64, i64) {
        let song_pk: i64 = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, source_index) VALUES ($1, $2) RETURNING id",
            "maimai_song",
            0
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_a: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, $2, 'dx', 'master', 0) RETURNING id",
            song_pk,
            "maimai_song|dx|master"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_b: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, $2, 'dx', 'expert', 1) RETURNING id",
            song_pk,
            "maimai_song|dx|expert"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        (song_pk, sheet_a, sheet_b)
    }

    #[sqlx::test]
    async fn fetch_all_sheets_groups_by_song(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, _a, _b) = seed_song_with_two_sheets(&pool).await;

        let grouped = fetch_all_sheets(&pool, None).await.unwrap();

        assert_eq!(grouped.get(&song_pk).map(|v| v.len()), Some(2));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_all_sheets_filters_by_region_keeps_empty_vec(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, sheet_a, _sheet_b) = seed_song_with_two_sheets(&pool).await;
        // Only sheet_a is available in "jp"; sheet_b has no region row at all.
        sqlx::query!(
            "INSERT INTO sheet_regions (sheet_id, region, available) VALUES ($1, 'jp', true)",
            sheet_a
        )
        .execute(&pool)
        .await?;

        let grouped = fetch_all_sheets(&pool, Some("jp")).await.unwrap();
        assert_eq!(grouped.get(&song_pk).map(|v| v.len()), Some(1));

        let grouped_kr = fetch_all_sheets(&pool, Some("kr")).await.unwrap();
        // Song must still be absent from the map (no sheets matched) — the
        // handler (a later ticket) is responsible for still emitting the song
        // with an empty sheets: [] array; fetch_all_sheets just doesn't
        // produce a key for songs with zero matching sheets.
        assert!(grouped_kr.get(&song_pk).is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheets_for_song_ignores_region(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, _a, _b) = seed_song_with_two_sheets(&pool).await;

        let sheets = fetch_sheets_for_song(&pool, song_pk).await.unwrap();

        assert_eq!(sheets.len(), 2);
        Ok(())
    }

    #[sqlx::test]
    async fn sheet_row_reports_has_chart(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, sheet_a, _sheet_b) = seed_song_with_two_sheets(&pool).await;
        sqlx::query!(
            "INSERT INTO charts (sheet_id, sheet_expr, content) VALUES ($1, $2, 'chart text')",
            sheet_a,
            "maimai_song|dx|master"
        )
        .execute(&pool)
        .await?;

        let sheets = fetch_sheets_for_song(&pool, song_pk).await.unwrap();
        let a = sheets.iter().find(|s| s.difficulty.as_deref() == Some("master")).unwrap();
        let b = sheets.iter().find(|s| s.difficulty.as_deref() == Some("expert")).unwrap();

        assert!(a.has_chart);
        assert!(!b.has_chart);
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_song_by_song_id_returns_none_when_missing(pool: PgPool) -> sqlx::Result<()> {
        let result = fetch_song_by_song_id(&pool, "does_not_exist").await.unwrap();
        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_song_by_song_id_finds_row(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_song_by_song_id(&pool, "maimai_song").await.unwrap();

        assert_eq!(result.map(|r| r.title), Some(Some("Example Song".to_string())));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheet_by_expr_returns_none_when_missing(pool: PgPool) -> sqlx::Result<()> {
        let result = fetch_sheet_by_expr(&pool, "nope|dx|master").await.unwrap();
        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheet_by_expr_joins_song_and_sheet(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master",
            "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = fetch_sheet_by_expr(&pool, "maimai_song|dx|master").await.unwrap();

        let (song, sheet) = result.expect("sheet should be found");
        assert_eq!(song.title.as_deref(), Some("Example Song"));
        assert_eq!(sheet.difficulty.as_deref(), Some("master"));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_all_songs_orders_by_source_index(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "second", "Second", 1
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "first", "First", 0
        )
        .execute(&pool)
        .await?;

        let songs = fetch_all_songs(&pool).await.unwrap();

        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title.as_deref(), Some("First"));
        assert_eq!(songs[1].title.as_deref(), Some("Second"));
        Ok(())
    }
}
