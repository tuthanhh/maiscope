use sqlx::PgPool;
use std::collections::HashMap;

pub struct SheetSearchParams {
    pub title: Option<String>,
    pub match_exact_title: bool,
    pub artist: Option<String>,
    pub match_exact_artist: bool,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub types: Vec<String>,
    pub difficulties: Vec<String>,
    pub min_level_value: Option<f64>,
    pub max_level_value: Option<f64>,
    pub use_internal_level: bool,
    pub min_bpm: Option<f64>,
    pub max_bpm: Option<f64>,
    pub note_designers: Vec<String>,
    pub region: Option<String>,
    pub use_region_override: bool,
    pub page: i64,
    pub page_size: i64,
}

impl Default for SheetSearchParams {
    fn default() -> Self {
        Self {
            title: None,
            match_exact_title: false,
            artist: None,
            match_exact_artist: false,
            categories: Vec::new(),
            versions: Vec::new(),
            types: Vec::new(),
            difficulties: Vec::new(),
            min_level_value: None,
            max_level_value: None,
            use_internal_level: false,
            min_bpm: None,
            max_bpm: None,
            note_designers: Vec::new(),
            region: None,
            use_region_override: false,
            page: 1,
            page_size: 22,
        }
    }
}

pub async fn search_sheets(
    pool: &PgPool,
    params: &SheetSearchParams,
) -> Result<(Vec<(crate::types::SongRow, crate::types::SheetRow)>, i64), sqlx::Error> {
    use sqlx::QueryBuilder;

    // Region-override-aware effective columns: when a positive region is
    // selected and use_region_override is set, the override's non-null
    // columns win; otherwise fall back to the sheet's own value.
    let effective_region = params
        .region
        .as_deref()
        .filter(|r| params.use_region_override && !r.starts_with('!'));

    fn build_where(
        qb: &mut QueryBuilder<sqlx::Postgres>,
        params: &SheetSearchParams,
        effective_region: Option<&str>,
    ) {
        qb.push(" WHERE 1=1 ");

        if let Some(title) = &params.title {
            if params.match_exact_title {
                qb.push(" AND so.title = ").push_bind(title.clone());
            } else {
                qb.push(" AND so.title ILIKE ")
                    .push_bind(format!("%{title}%"));
            }
        }
        if let Some(artist) = &params.artist {
            if params.match_exact_artist {
                qb.push(" AND so.artist = ").push_bind(artist.clone());
            } else {
                qb.push(" AND so.artist ILIKE ")
                    .push_bind(format!("%{artist}%"));
            }
        }
        if !params.categories.is_empty() {
            qb.push(" AND string_to_array(so.category, '|') && ")
                .push_bind(params.categories.clone());
        }
        if !params.versions.is_empty() {
            qb.push(" AND so.version = ANY(")
                .push_bind(params.versions.clone())
                .push(")");
        }
        if !params.types.is_empty() {
            qb.push(" AND s.type = ANY(")
                .push_bind(params.types.clone())
                .push(")");
        }
        if !params.difficulties.is_empty() {
            qb.push(" AND s.difficulty = ANY(")
                .push_bind(params.difficulties.clone())
                .push(")");
        }
        if !params.note_designers.is_empty() {
            let designer_col = if effective_region.is_some() {
                "COALESCE(sro.note_designer, s.note_designer)"
            } else {
                "s.note_designer"
            };
            qb.push(" AND ")
                .push(designer_col)
                .push(" = ANY(")
                .push_bind(params.note_designers.clone())
                .push(")");
        }
        let level_col = match (params.use_internal_level, effective_region.is_some()) {
            (true, true) => "COALESCE(sro.internal_level_value, s.internal_level_value)",
            (true, false) => "s.internal_level_value",
            (false, true) => "COALESCE(sro.level_value, s.level_value)",
            (false, false) => "s.level_value",
        };
        if let Some(min_level) = params.min_level_value {
            qb.push(" AND ")
                .push(level_col)
                .push(" >= ")
                .push_bind(min_level);
        }
        if let Some(max_level) = params.max_level_value {
            qb.push(" AND ")
                .push(level_col)
                .push(" <= ")
                .push_bind(max_level);
        }
        if let Some(min_bpm) = params.min_bpm {
            qb.push(" AND so.bpm >= ").push_bind(min_bpm);
        }
        if let Some(max_bpm) = params.max_bpm {
            qb.push(" AND so.bpm <= ").push_bind(max_bpm);
        }
        if let Some(region) = &params.region {
            if let Some(excluded) = region.strip_prefix('!') {
                qb.push(" AND (sr_excl.available IS NULL OR sr_excl.available = false) ");
                let _ = excluded; // the join binds the region value, see build_joins
            } else if effective_region.is_some() {
                // A region override row is itself evidence the sheet belongs
                // to this region (overrides only make sense for sheets
                // available there), so it counts as inclusion even when no
                // separate sheet_regions availability row exists.
                qb.push(" AND (sr_incl.available = true OR sro.sheet_id IS NOT NULL) ");
            } else {
                qb.push(" AND sr_incl.available = true ");
            }
        }
    }

    fn build_joins(
        qb: &mut QueryBuilder<sqlx::Postgres>,
        params: &SheetSearchParams,
        effective_region: Option<&str>,
    ) {
        if let Some(region) = effective_region {
            qb.push(
                " LEFT JOIN sheet_region_overrides sro ON sro.sheet_id = s.id AND sro.region = ",
            )
            .push_bind(region.to_string());
        }
        if let Some(region) = &params.region {
            if let Some(excluded) = region.strip_prefix('!') {
                qb.push(" LEFT JOIN sheet_regions sr_excl ON sr_excl.sheet_id = s.id AND sr_excl.region = ")
                    .push_bind(excluded.to_string());
            } else {
                qb.push(" LEFT JOIN sheet_regions sr_incl ON sr_incl.sheet_id = s.id AND sr_incl.region = ")
                    .push_bind(region.clone());
            }
        }
    }

    // ── total count ──
    let mut count_qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM sheets s JOIN songs so ON so.id = s.song_id_fk ");
    build_joins(&mut count_qb, params, effective_region);
    build_where(&mut count_qb, params, effective_region);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    // ── page of rows ──
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        r#"SELECT
            so.id, so.song_id, so.category, so.title, so.artist, so.bpm, so.image_name, so.version,
            to_char(so.release_date, 'YYYY-MM-DD') AS release_date, so.is_new, so.is_locked, so.comment,
            s.song_id_fk, s.type, s.difficulty, s.level, s.level_value, s.internal_level,
            s.internal_level_value, s.note_designer, s.is_special,
            EXISTS (SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL) AS has_chart,
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id) AS note_counts,
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id) AS regions,
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id) AS region_overrides
           FROM sheets s JOIN songs so ON so.id = s.song_id_fk "#,
    );
    build_joins(&mut qb, params, effective_region);
    build_where(&mut qb, params, effective_region);
    // Newest-first, matching GET /catalog's convention (client used to
    // achieve this via data.songs.reverse() on the full catalog; this
    // endpoint returns an already-paginated page, so it must sort this way
    // itself instead of relying on a reversal step that no longer runs).
    qb.push(" ORDER BY so.source_index DESC, s.source_index ");
    qb.push(" LIMIT ").push_bind(params.page_size);
    qb.push(" OFFSET ")
        .push_bind((params.page - 1).max(0) * params.page_size);

    let rows = qb.build_query_as::<SearchRow>().fetch_all(pool).await?;

    let result = rows
        .into_iter()
        .map(|row| {
            let song = crate::types::SongRow {
                id: row.id,
                song_id: row.song_id,
                category: row.category,
                title: row.title,
                artist: row.artist,
                bpm: row.bpm,
                image_name: row.image_name,
                version: row.version,
                release_date: row.release_date,
                is_new: row.is_new,
                is_locked: row.is_locked,
                comment: row.comment,
            };
            let sheet = crate::types::SheetRow {
                song_id_fk: row.song_id_fk,
                r#type: row.r#type,
                difficulty: row.difficulty,
                level: row.level,
                level_value: row.level_value,
                internal_level: row.internal_level,
                internal_level_value: row.internal_level_value,
                note_designer: row.note_designer,
                is_special: row.is_special,
                has_chart: row.has_chart,
                note_counts: row.note_counts,
                regions: row.regions,
                region_overrides: row.region_overrides,
            };
            (song, sheet)
        })
        .collect();

    Ok((result, total))
}

/// Flat decode target for `search_sheets`'s dynamic query — every column from
/// both `songs` and `sheets` in one row. `QueryBuilder`'s runtime query has no
/// compile-time column-order checking, so `FromRow`'s by-name matching (not
/// `query_as!`'s positional matching) is what keeps this correct.
#[derive(sqlx::FromRow)]
struct SearchRow {
    id: i64,
    song_id: Option<String>,
    category: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    bpm: Option<f64>,
    image_name: Option<String>,
    version: Option<String>,
    release_date: Option<String>,
    is_new: Option<bool>,
    is_locked: Option<bool>,
    comment: Option<String>,
    song_id_fk: i64,
    r#type: Option<String>,
    difficulty: Option<String>,
    level: Option<String>,
    level_value: Option<f64>,
    internal_level: Option<String>,
    internal_level_value: Option<f64>,
    note_designer: Option<String>,
    is_special: Option<bool>,
    has_chart: bool,
    note_counts: Option<NoteCountsJson>,
    regions: Option<RegionsJson>,
    region_overrides: Option<RegionOverridesJson>,
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
type RegionOverridesJson =
    sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let a = sheets
            .iter()
            .find(|s| s.difficulty.as_deref() == Some("master"))
            .unwrap();
        let b = sheets
            .iter()
            .find(|s| s.difficulty.as_deref() == Some("expert"))
            .unwrap();

        assert!(a.has_chart);
        assert!(!b.has_chart);
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

        let result = fetch_sheet_by_expr(&pool, "maimai_song|dx|master")
            .await
            .unwrap();

        let (song, sheet) = result.expect("sheet should be found");
        assert_eq!(song.title.as_deref(), Some("Example Song"));
        assert_eq!(sheet.difficulty.as_deref(), Some("master"));
        Ok(())
    }

    async fn seed_two_songs_for_search(pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, artist, category, version, bpm, source_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            "song_a",
            "Fire Flower",
            "Composer A",
            "pops",
            "maimai DX",
            180.0_f64,
            0
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO songs (song_id, title, artist, category, version, bpm, source_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            "song_b",
            "Ice Crystal",
            "Composer B",
            "anime",
            "maimai DX",
            140.0_f64,
            1
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level_value, source_index)
             SELECT id, $1, 'dx', 'master', 13.5, 0 FROM songs WHERE song_id = $2",
            "song_a|dx|master", "song_a"
        )
        .execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level_value, source_index)
             SELECT id, $1, 'dx', 'master', 11.0, 0 FROM songs WHERE song_id = $2",
            "song_b|dx|master", "song_b"
        )
        .execute(pool).await.unwrap();
    }

    #[sqlx::test]
    async fn search_sheets_filters_by_title_substring(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams {
            title: Some("fire".to_string()),
            ..Default::default()
        };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Fire Flower"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_filters_by_level_range(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams {
            min_level_value: Some(12.0),
            ..Default::default()
        };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Fire Flower"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_paginates_and_reports_total(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams {
            page: 1,
            page_size: 1,
            ..Default::default()
        };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 2); // total matches regardless of page_size
        assert_eq!(rows.len(), 1);
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_orders_newest_first(pool: PgPool) -> sqlx::Result<()> {
        // seed_two_songs_for_search inserts song_a (source_index 0) then
        // song_b (source_index 1) — source_index is upstream data.json order
        // (oldest-added first), so song_b is newer. GET /catalog's client-side
        // buildCatalog reverses to newest-first; this endpoint must match that
        // convention itself, since results here aren't reversed client-side.
        seed_two_songs_for_search(&pool).await;

        let (rows, _total) = search_sheets(&pool, &SheetSearchParams::default())
            .await
            .unwrap();

        assert_eq!(rows[0].0.title.as_deref(), Some("Ice Crystal"));
        assert_eq!(rows[1].0.title.as_deref(), Some("Fire Flower"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_matches_pipe_delimited_category(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, category, source_index) VALUES ($1, $2, $3, $4)",
            "song_c",
            "Dual Genre",
            "pops|anime",
            0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, source_index)
             SELECT id, $1, 0 FROM songs WHERE song_id = $2",
            "song_c|dx|master",
            "song_c"
        )
        .execute(&pool)
        .await?;

        let params = SheetSearchParams {
            categories: vec!["anime".to_string()],
            ..Default::default()
        };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Dual Genre"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_applies_region_override_before_level_filter(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "song_d",
            "Region Song",
            0
        )
        .execute(&pool)
        .await?;
        let sheet_pk: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, level_value, source_index)
             SELECT id, $1, 10.0, 0 FROM songs WHERE song_id = $2 RETURNING id",
            "song_d|dx|master",
            "song_d"
        )
        .fetch_one(&pool)
        .await?;
        // In the "jp" region this sheet is actually level 14.0.
        sqlx::query!(
            "INSERT INTO sheet_region_overrides (sheet_id, region, level_value) VALUES ($1, 'jp', 14.0)",
            sheet_pk
        )
        .execute(&pool).await?;

        // Without the override: min_level_value 12 excludes it (base is 10.0).
        let base_params = SheetSearchParams {
            min_level_value: Some(12.0),
            ..Default::default()
        };
        let (_, total_base) = search_sheets(&pool, &base_params).await.unwrap();
        assert_eq!(total_base, 0);

        // With region=jp + useRegionOverride: the override's 14.0 satisfies min 12.
        let override_params = SheetSearchParams {
            min_level_value: Some(12.0),
            region: Some("jp".to_string()),
            use_region_override: true,
            ..Default::default()
        };
        let (_, total_override) = search_sheets(&pool, &override_params).await.unwrap();
        assert_eq!(total_override, 1);
        Ok(())
    }
}
