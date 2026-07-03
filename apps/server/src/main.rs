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

// GET /catalog — assembles the full Data shape (types/Data.ts) from the DB.
// Byte-compatible with the old data.json so preprocessData is unchanged.
async fn catalog(
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let data: Value = sqlx::query_scalar(
        "SELECT json_build_object(
            'songs',        (SELECT COALESCE(json_agg(doc ORDER BY source_index), '[]') FROM v_song),
            'categories',   (SELECT COALESCE(json_agg(json_build_object('category', category) ORDER BY ordinal), '[]') FROM categories),
            'versions',     (SELECT COALESCE(json_agg(json_build_object('version', version, 'abbr', abbr, 'releaseDate', to_char(release_date, 'YYYY-MM-DD')) ORDER BY ordinal), '[]') FROM versions),
            'types',        (SELECT COALESCE(json_agg(json_build_object('type', type, 'name', name, 'abbr', abbr, 'iconUrl', icon_url, 'iconHeight', icon_height) ORDER BY ordinal), '[]') FROM types),
            'difficulties', (SELECT COALESCE(json_agg(json_build_object('difficulty', difficulty, 'name', name, 'color', color, 'iconUrl', icon_url, 'iconHeight', icon_height) ORDER BY ordinal), '[]') FROM difficulties),
            'regions',      (SELECT COALESCE(json_agg(json_build_object('region', region, 'name', name) ORDER BY ordinal), '[]') FROM regions),
            'updateTime',   (SELECT to_char(update_time, 'YYYY-MM-DD') FROM catalog_meta LIMIT 1)
        )",
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(data))
}

// GET /songs/{id} — single Song with its sheets (types/Song.ts).
async fn get_song(
    Path(id): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let doc: Option<Value> = sqlx::query_scalar("SELECT doc FROM v_song WHERE song_id = $1")
        .bind(&id)
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?;

    doc.map(Json).ok_or_else(|| not_found("song", &id))
}

// GET /sheets/{sheetExpr} — standalone Sheet, song fields flattened in
// (types/Sheet.ts). sheetExpr is URL-encoded; axum decodes the path segment.
async fn get_sheet(
    Path(sheet_expr): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let doc: Option<Value> = sqlx::query_scalar("SELECT doc FROM v_sheet WHERE sheet_expr = $1")
        .bind(&sheet_expr)
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?;

    doc.map(Json).ok_or_else(|| not_found("sheet", &sheet_expr))
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
