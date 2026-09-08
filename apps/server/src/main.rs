mod queries;
mod types;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
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
                .route("/catalog", get(catalog))
                .route("/songs/{id}", get(get_song))
                .route("/sheets/{sheet}", get(get_sheet))
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

// Query params for GET /catalog (contract §1). `since` (sync-tier revision
// check) is not implemented yet — no §3 sync tier exists.
#[derive(Debug, Deserialize)]
struct CatalogQuery {
    region: Option<String>,
}

// GET /catalog — assembles the full Data shape (types/Data.ts) from the DB.
// Byte-compatible with the old data.json so preprocessData is unchanged.
async fn catalog(
    Query(CatalogQuery { region }): Query<CatalogQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Catalog>, (StatusCode, Json<Value>)> {
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

    Ok(Json(catalog))
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

        let result = catalog(
            AxumQuery(CatalogQuery { region: Some("kr".to_string()) }),
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(result.0.songs.len(), 1);
        assert_eq!(result.0.songs[0].sheets.len(), 0);
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
}
