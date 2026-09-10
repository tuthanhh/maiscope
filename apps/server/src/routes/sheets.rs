use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sheets/{sheet}", get(get_sheet))
        .route("/sheets/search", get(search_sheets))
}

// GET /sheets/{sheetExpr} — standalone Sheet, song fields flattened in
// (types/Sheet.ts). sheetExpr is URL-encoded; axum decodes the path segment.
async fn get_sheet(
    Path(sheet_expr): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<types::Sheet>, AppError> {
    let Some((song_row, sheet_row)) = queries::fetch_sheet_by_expr(&pool, &sheet_expr).await?
    else {
        return Err(AppError::NotFound {
            kind: "sheet",
            key: sheet_expr,
        });
    };

    Ok(Json(types::Sheet {
        song: song_row.into_meta(),
        sheet: sheet_row.into_meta(),
    }))
}

// Query params for GET /sheets/search — mirrors apps/host/src/types/Filters.ts
// (minus superFilter, which is being removed from the app — see the
// companion frontend plan).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SheetSearchQuery {
    title: Option<String>,
    #[serde(default)]
    match_exact_title: bool,
    artist: Option<String>,
    #[serde(default)]
    match_exact_artist: bool,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    difficulties: Vec<String>,
    min_level_value: Option<f64>,
    max_level_value: Option<f64>,
    #[serde(default)]
    use_internal_level: bool,
    min_bpm: Option<f64>,
    max_bpm: Option<f64>,
    #[serde(default)]
    note_designers: Vec<String>,
    region: Option<String>,
    #[serde(default)]
    use_region_override: bool,
    #[serde(default = "default_search_page")]
    page: i64,
    #[serde(default = "default_search_page_size")]
    page_size: i64,
}

fn default_search_page() -> i64 {
    1
}
fn default_search_page_size() -> i64 {
    22
}

// GET /sheets/search — filtered, paginated sheet list (contract §1.2).
async fn search_sheets(
    Query(q): Query<SheetSearchQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<types::SheetSearchResponse>, AppError> {
    let params = queries::SheetSearchParams {
        title: q.title,
        match_exact_title: q.match_exact_title,
        artist: q.artist,
        match_exact_artist: q.match_exact_artist,
        categories: q.categories,
        versions: q.versions,
        types: q.types,
        difficulties: q.difficulties,
        min_level_value: q.min_level_value,
        max_level_value: q.max_level_value,
        use_internal_level: q.use_internal_level,
        min_bpm: q.min_bpm,
        max_bpm: q.max_bpm,
        note_designers: q.note_designers,
        region: q.region,
        use_region_override: q.use_region_override,
        page: q.page.max(1),
        page_size: q.page_size.clamp(1, 100),
    };

    let (rows, total) = queries::search_sheets(&pool, &params).await?;

    let sheets = rows
        .into_iter()
        .map(|(song, sheet)| types::Sheet {
            song: song.into_meta(),
            sheet: sheet.into_meta(),
        })
        .collect();

    Ok(Json(types::SheetSearchResponse { sheets, total }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query as AxumQuery, State as AxumState};

    #[sqlx::test]
    async fn get_sheet_returns_404_for_unknown_expr(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let result = get_sheet(Path("nope|dx|master".to_string()), AxumState(pool)).await;
        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn get_sheet_flattens_song_and_sheet_fields(pool: sqlx::PgPool) -> sqlx::Result<()> {
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

        let result = get_sheet(Path("maimai_song|dx|master".to_string()), AxumState(pool))
            .await
            .unwrap();

        assert_eq!(result.0.song.title.as_deref(), Some("Example Song"));
        assert_eq!(result.0.sheet.difficulty.as_deref(), Some("master"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_handler_returns_paginated_response(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, source_index)
             SELECT id, $1, 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master",
            "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = search_sheets(
            AxumQuery(SheetSearchQuery {
                title: None,
                match_exact_title: false,
                artist: None,
                match_exact_artist: false,
                categories: vec![],
                versions: vec![],
                types: vec![],
                difficulties: vec![],
                min_level_value: None,
                max_level_value: None,
                use_internal_level: false,
                min_bpm: None,
                max_bpm: None,
                note_designers: vec![],
                region: None,
                use_region_override: false,
                page: 1,
                page_size: 22,
            }),
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(result.0.total, 1);
        assert_eq!(result.0.sheets.len(), 1);
        Ok(())
    }
}
