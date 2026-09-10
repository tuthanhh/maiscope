use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::routes::caching::{CATALOG_CACHE_CONTROL, Freshness};
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

// GET /catalog — assembles the full Data shape (types/Data.ts) from the DB.
// Byte-compatible with the old data.json so preprocessData is unchanged.
// Supports ETag/If-None-Match (contract §1) sharing the sync tier's
// catalogHash concept (see sync_manifest).
async fn catalog(
    Query(CatalogQuery { region }): Query<CatalogQuery>,
    headers: axum::http::HeaderMap,
    State(pool): State<Pool<Postgres>>,
) -> Result<axum::response::Response, AppError> {
    let freshness = Freshness::load(&pool).await?;
    if freshness.is_current_for(&headers) {
        return Ok(freshness.not_modified(CATALOG_CACHE_CONTROL));
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

    Ok((freshness.headers(CATALOG_CACHE_CONTROL), Json(catalog)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query as AxumQuery, State as AxumState};
    use axum::http::StatusCode;
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

    async fn seed_minimal_catalog(pool: &sqlx::PgPool) {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(pool)
            .await
            .unwrap();
    }

    #[sqlx::test]
    async fn catalog_returns_304_when_if_none_match_matches(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_minimal_catalog(&pool).await;

        let first = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool.clone()),
        )
        .await
        .unwrap();
        let etag = first
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .clone();

        let mut conditional_headers = axum::http::HeaderMap::new();
        conditional_headers.insert(axum::http::header::IF_NONE_MATCH, etag);

        let second = catalog(
            AxumQuery(CatalogQuery { region: None }),
            conditional_headers,
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        let body = axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty());
        Ok(())
    }

    // RFC 7232 §4.1: a 304 must send the same validators the 200 would have,
    // because the client uses them to refresh what it already has stored.
    // Dropping Cache-Control here throws away ticket 07's max-age on every
    // revalidation — the cached copy goes stale again immediately and the
    // next visit re-revalidates instead of being served locally.
    #[sqlx::test]
    async fn catalog_304_repeats_the_etag_and_cache_control(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_minimal_catalog(&pool).await;

        let full = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool.clone()),
        )
        .await
        .unwrap();
        let etag = full.headers().get(axum::http::header::ETAG).unwrap().clone();
        let cache_control = full
            .headers()
            .get(axum::http::header::CACHE_CONTROL)
            .unwrap()
            .clone();

        let mut conditional_headers = axum::http::HeaderMap::new();
        conditional_headers.insert(axum::http::header::IF_NONE_MATCH, etag.clone());

        let not_modified = catalog(
            AxumQuery(CatalogQuery { region: None }),
            conditional_headers,
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            not_modified.headers().get(axum::http::header::ETAG),
            Some(&etag)
        );
        assert_eq!(
            not_modified.headers().get(axum::http::header::CACHE_CONTROL),
            Some(&cache_control)
        );
        Ok(())
    }

    // The negative half of the conditional. Every other 304 test sends a
    // matching validator, so an `is_current_for` that was inverted or always
    // true would leave them all green while serving a permanent 304 — every
    // client stuck on whatever catalog it first fetched, with no way to
    // learn the catalog had changed.
    #[sqlx::test]
    async fn catalog_serves_a_body_when_if_none_match_is_stale(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        seed_minimal_catalog(&pool).await;

        let mut stale = axum::http::HeaderMap::new();
        stale.insert(
            axum::http::header::IF_NONE_MATCH,
            axum::http::HeaderValue::from_static("\"not-the-current-hash\""),
        );

        let response = catalog(
            AxumQuery(CatalogQuery { region: None }),
            stale,
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["songs"].as_array().unwrap().len(), 1);
        Ok(())
    }

    #[sqlx::test]
    async fn catalog_etag_changes_when_catalog_meta_changes(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_minimal_catalog(&pool).await;

        let first = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool.clone()),
        )
        .await
        .unwrap();
        let etag_before = first
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .clone();

        sqlx::query!("UPDATE catalog_meta SET revision = revision + 1")
            .execute(&pool)
            .await?;

        let second = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool),
        )
        .await
        .unwrap();
        let etag_after = second
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .clone();

        assert_ne!(etag_before, etag_after);
        Ok(())
    }
}
