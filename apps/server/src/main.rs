mod queries;
mod types;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Pool, Postgres};
use tower_http::cors::CorsLayer;
use types::{Catalog, NestedSheet, Song};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Difficulty {
    Basic,
    Advanced,
    Expert,
    Master,
    ReMaster,
}

impl Difficulty {
    // The code stored in sheets.difficulty / sheet_expr.
    fn code(&self) -> &'static str {
        match self {
            Difficulty::Basic => "basic",
            Difficulty::Advanced => "advanced",
            Difficulty::Expert => "expert",
            Difficulty::Master => "master",
            Difficulty::ReMaster => "remaster",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ChartType {
    Dx,
    Std,
    Utage,
}

impl ChartType {
    fn code(&self) -> &'static str {
        match self {
            ChartType::Dx => "dx",
            ChartType::Std => "std",
            ChartType::Utage => "utage",
        }
    }
}

// Query params for GET /sheets/{sheet}/chart, e.g. ?type=dx&difficulty=master.
// type is required: a song has separate dx/std sheets at the same difficulty.
#[derive(Debug, Deserialize)]
struct ChartQuery {
    r#type: ChartType,
    difficulty: Difficulty,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap();

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/healthcheck", get(healthcheck))
                .route("/sync/manifest", get(sync_manifest))
                .route("/sync/delta", get(sync_delta))
                .route("/catalog", get(catalog))
                .route("/songs/{id}", get(get_song))
                .route("/sheets/{sheet}", get(get_sheet))
                .route("/sheets/search", get(search_sheets))
                .route("/sheets/{sheet}/chart", get(get_chart)),
        )
        // Browser dev build (Vite) hits this cross-origin; Tauri routes through
        // src-tauri so it doesn't need CORS. Permissive is fine for local dev.
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.expect("server crashed");
}

async fn healthcheck() -> Json<Value> {
    Json(json!({ "status": "good" }))
}

// Maps any sqlx error to a 500 with the contract error shape.
fn db_error(e: sqlx::Error) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "database_error", "message": e.to_string() })),
    )
}

fn not_found(kind: &str, key: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "not_found", "message": format!("{kind} '{key}' not found") })),
    )
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
) -> Result<Json<types::SheetSearchResponse>, (StatusCode, Json<Value>)> {
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

    let (rows, total) = queries::search_sheets(&pool, &params).await.map_err(db_error)?;

    let sheets = rows
        .into_iter()
        .map(|(song, sheet)| types::Sheet { song: song.into_meta(), sheet: sheet.into_meta() })
        .collect();

    Ok(Json(types::SheetSearchResponse { sheets, total }))
}

// Query params for GET /catalog (contract §1).
#[derive(Debug, Deserialize)]
struct CatalogQuery {
    region: Option<String>,
}

fn catalog_hash(revision: i64, update_time: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{revision}:{update_time}").as_bytes());
    format!("{:x}", hasher.finalize())
}

// GET /catalog — assembles the full Data shape (types/Data.ts) from the DB.
// Byte-compatible with the old data.json so preprocessData is unchanged.
// Supports ETag/If-None-Match (contract §1) sharing the sync tier's
// catalogHash concept (see sync_manifest).
async fn catalog(
    Query(CatalogQuery { region }): Query<CatalogQuery>,
    headers: axum::http::HeaderMap,
    State(pool): State<Pool<Postgres>>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let meta = sqlx::query!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
           FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;
    let etag = format!("\"{}\"", catalog_hash(meta.revision, &meta.update_time));

    if headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag.as_str())
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    let song_rows = queries::fetch_all_songs(&pool).await.map_err(db_error)?;
    let mut sheets_by_song = queries::fetch_all_sheets(&pool, region.as_deref())
        .await
        .map_err(db_error)?;

    let songs = song_rows
        .into_iter()
        .map(|row| {
            let sheets = sheets_by_song
                .remove(&row.id)
                .unwrap_or_default()
                .into_iter()
                .map(|sheet_row| NestedSheet { sheet: sheet_row.into_meta() })
                .collect();
            Song { meta: row.into_meta(), sheets }
        })
        .collect();

    let catalog = Catalog {
        songs,
        categories: queries::fetch_categories(&pool).await.map_err(db_error)?,
        versions: queries::fetch_versions(&pool).await.map_err(db_error)?,
        types: queries::fetch_types(&pool).await.map_err(db_error)?,
        difficulties: queries::fetch_difficulties(&pool).await.map_err(db_error)?,
        regions: queries::fetch_regions(&pool).await.map_err(db_error)?,
        update_time: queries::fetch_update_time(&pool).await.map_err(db_error)?,
    };

    Ok(([(axum::http::header::ETAG, etag)], Json(catalog)).into_response())
}

// GET /songs/{id} — single Song with its sheets (types/Song.ts).
async fn get_song(
    Path(id): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Song>, (StatusCode, Json<Value>)> {
    let Some(song_row) = queries::fetch_song_by_song_id(&pool, &id)
        .await
        .map_err(db_error)?
    else {
        return Err(not_found("song", &id));
    };

    let sheet_rows = queries::fetch_sheets_for_song(&pool, song_row.id)
        .await
        .map_err(db_error)?;

    let sheets = sheet_rows
        .into_iter()
        .map(|row| NestedSheet { sheet: row.into_meta() })
        .collect();

    Ok(Json(Song { meta: song_row.into_meta(), sheets }))
}

// GET /sheets/{sheetExpr} — standalone Sheet, song fields flattened in
// (types/Sheet.ts). sheetExpr is URL-encoded; axum decodes the path segment.
async fn get_sheet(
    Path(sheet_expr): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<types::Sheet>, (StatusCode, Json<Value>)> {
    let Some((song_row, sheet_row)) = queries::fetch_sheet_by_expr(&pool, &sheet_expr)
        .await
        .map_err(db_error)?
    else {
        return Err(not_found("sheet", &sheet_expr));
    };

    Ok(Json(types::Sheet { song: song_row.into_meta(), sheet: sheet_row.into_meta() }))
}

// GET /sheets/{songId}/chart?type=dx&difficulty=master
// Returns raw simai chart text (text/plain) for the wasm engine to parse.
// On error, body is a plain message and the status code conveys the kind.
async fn get_chart(
    Path(song_id): Path<String>,
    Query(ChartQuery { r#type, difficulty }): Query<ChartQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<String, (StatusCode, String)> {
    // sheet_expr = songId|type|difficulty (utils/sheet.ts:computeSheetExpr).
    let sheet_expr = format!("{song_id}|{}|{}", r#type.code(), difficulty.code());

    let content: Option<Option<String>> = sqlx::query_scalar(
        "SELECT c.content FROM charts c \
         JOIN sheets s ON s.id = c.sheet_id \
         WHERE s.sheet_expr = $1 AND c.format = 'maimai-simai'",
    )
    .bind(&sheet_expr)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match content {
        // Inline simai present → 200 text/plain.
        Some(Some(text)) => Ok(text),
        // Row exists but no inline content (blob-only chart not yet supported).
        Some(None) => Err((
            StatusCode::NOT_IMPLEMENTED,
            format!("chart for '{sheet_expr}' has no inline simai content"),
        )),
        // No chart for this sheet.
        None => Err((
            StatusCode::NOT_FOUND,
            format!("no chart for '{sheet_expr}'"),
        )),
    }
}

// GET /sync/manifest — cheap freshness probe (contract §3).
async fn sync_manifest(
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let row = sqlx::query!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
           FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    let song_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM songs"#)
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;
    let sheet_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM sheets"#)
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;
    let chart_count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM charts WHERE content IS NOT NULL"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(json!({
        "updateTime": row.update_time,
        "revision": row.revision,
        "catalogHash": catalog_hash(row.revision, &row.update_time),
        "counts": { "songs": song_count, "sheets": sheet_count, "charts": chart_count }
    })))
}

// Query params for GET /sync/delta?since={revision} (contract §3).
#[derive(Debug, Deserialize)]
struct DeltaQuery {
    since: i64,
}

// GET /sync/delta?since={revision} — rows changed since `revision`, or `409
// snapshot_required` if `since` predates the last full ingest reload (contract
// §3). Only bin/ingest's full reloads and (future) contribution-approve edits
// bump revision, so `since` "too old to diff" is defined precisely against
// `last_full_reload_revision`, not a vague staleness heuristic.
async fn sync_delta(
    Query(q): Query<DeltaQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let last_full_reload: i64 = sqlx::query_scalar!(
        r#"SELECT last_full_reload_revision AS "v!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    if q.since < last_full_reload {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({ "error": "snapshot_required" })),
        ));
    }

    // A song counts as "changed" if it changed itself OR any of its sheets
    // did — a sheet-only edit (e.g. a future targeted contribution-approve)
    // must still surface the song, since the response nests sheets under it.
    let changed_song_ids: Vec<i64> = sqlx::query_scalar!(
        r#"SELECT DISTINCT so.id AS "id!" FROM songs so
           LEFT JOIN sheets s ON s.song_id_fk = so.id
           WHERE so.revision > $1 OR s.revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    let mut songs = Vec::with_capacity(changed_song_ids.len());
    for song_pk in changed_song_ids {
        let song_row = sqlx::query_as!(
            types::SongRow,
            r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                      to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
               FROM songs WHERE id = $1"#,
            song_pk
        )
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;
        let sheet_rows = queries::fetch_sheets_for_song(&pool, song_pk)
            .await
            .map_err(db_error)?;
        let sheets = sheet_rows
            .into_iter()
            .map(|r| NestedSheet { sheet: r.into_meta() })
            .collect();
        songs.push(Song { meta: song_row.into_meta(), sheets });
    }

    let current_revision: i64 = sqlx::query_scalar!(r#"SELECT revision AS "v!" FROM catalog_meta LIMIT 1"#)
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;

    let deleted_song_ids: Vec<String> = sqlx::query_scalar!(
        r#"SELECT song_id AS "v!" FROM deleted_songs WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;
    let deleted_sheet_exprs: Vec<String> = sqlx::query_scalar!(
        r#"SELECT sheet_expr AS "v!" FROM deleted_sheets WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(json!({
        "revision": current_revision,
        "songs": songs,
        "charts": [], // chart-meta delta tracking deferred — see api-contract.md §3
        "tombstones": { "songIds": deleted_song_ids, "sheetExprs": deleted_sheet_exprs }
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query as AxumQuery, State as AxumState};

    #[sqlx::test]
    async fn catalog_returns_song_with_empty_sheets_when_region_excludes_all(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool)
        .await?;
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        // No sheet_regions row for "kr" — sheet_a should be filtered out.

        let response = catalog(
            AxumQuery(CatalogQuery { region: Some("kr".to_string()) }),
            axum::http::HeaderMap::new(),
            AxumState(pool),
        )
        .await
        .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["songs"].as_array().unwrap().len(), 1);
        assert_eq!(json["songs"][0]["sheets"].as_array().unwrap().len(), 0);
        Ok(())
    }

    #[sqlx::test]
    async fn get_song_returns_404_for_unknown_id(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let result = get_song(Path("nope".to_string()), AxumState(pool)).await;
        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn get_song_returns_song_with_sheets(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = get_song(Path("maimai_song".to_string()), AxumState(pool)).await.unwrap();

        assert_eq!(result.0.sheets.len(), 1);
        Ok(())
    }

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
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
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
    async fn search_sheets_handler_returns_paginated_response(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool).await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, source_index)
             SELECT id, $1, 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool).await?;

        let result = search_sheets(
            AxumQuery(SheetSearchQuery {
                title: None, match_exact_title: false, artist: None, match_exact_artist: false,
                categories: vec![], versions: vec![], types: vec![], difficulties: vec![],
                min_level_value: None, max_level_value: None, use_internal_level: false,
                min_bpm: None, max_bpm: None, note_designers: vec![], region: None,
                use_region_override: false, page: 1, page_size: 22,
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
