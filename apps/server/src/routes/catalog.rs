use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types::{Catalog, NestedSheet, Song};

pub fn router() -> Router<AppState> {
    Router::new().route("/catalog", get(catalog))
}

// Query params for GET /catalog (contract §1).
#[derive(Debug, Deserialize)]
struct CatalogQuery {
    region: Option<String>,
}

// Shared with routes::sync (sync_manifest exposes the same hash).
pub(crate) fn catalog_hash(revision: i64, update_time: &str) -> String {
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
) -> Result<axum::response::Response, AppError> {
    let meta = sqlx::query!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
           FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;
    let etag = format!("\"{}\"", catalog_hash(meta.revision, &meta.update_time));

    if headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(etag.as_str())
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    let song_rows = queries::fetch_all_songs(&pool).await?;
    let mut sheets_by_song = queries::fetch_all_sheets(&pool, region.as_deref()).await?;

    let songs = song_rows
        .into_iter()
        .map(|row| {
            let sheets = sheets_by_song
                .remove(&row.id)
                .unwrap_or_default()
                .into_iter()
                .map(|sheet_row| NestedSheet {
                    sheet: sheet_row.into_meta(),
                })
                .collect();
            Song {
                meta: row.into_meta(),
                sheets,
            }
        })
        .collect();

    let catalog = Catalog {
        songs,
        categories: queries::fetch_categories(&pool).await?,
        versions: queries::fetch_versions(&pool).await?,
        types: queries::fetch_types(&pool).await?,
        difficulties: queries::fetch_difficulties(&pool).await?,
        regions: queries::fetch_regions(&pool).await?,
        update_time: queries::fetch_update_time(&pool).await?,
    };

    Ok(([(axum::http::header::ETAG, etag)], Json(catalog)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query as AxumQuery, State as AxumState};
    use serde_json::Value;

    #[sqlx::test]
    async fn catalog_returns_song_with_empty_sheets_when_region_excludes_all(
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
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master",
            "maimai_song"
        )
        .execute(&pool)
        .await?;
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        // No sheet_regions row for "kr" — sheet_a should be filtered out.

        let response = catalog(
            AxumQuery(CatalogQuery {
                region: Some("kr".to_string()),
            }),
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
}
