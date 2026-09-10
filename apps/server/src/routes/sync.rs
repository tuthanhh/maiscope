use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::routes::caching::{Freshness, MANIFEST_CACHE_CONTROL};
use crate::state::AppState;
use crate::types::{self, NestedSheet, Song};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync/manifest", get(sync_manifest))
        .route("/sync/delta", get(sync_delta))
}

// GET /sync/manifest — cheap freshness probe (contract §3).
async fn sync_manifest(
    headers: axum::http::HeaderMap,
    State(pool): State<Pool<Postgres>>,
) -> Result<axum::response::Response, AppError> {
    let freshness = Freshness::load(&pool).await?;
    if freshness.is_current_for(&headers) {
        return Ok(freshness.not_modified(MANIFEST_CACHE_CONTROL));
    }

    let song_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM songs"#)
        .fetch_one(&pool)
        .await?;
    let sheet_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM sheets"#)
        .fetch_one(&pool)
        .await?;
    let chart_count: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM charts WHERE content IS NOT NULL"#)
            .fetch_one(&pool)
            .await?;

    Ok((
        freshness.headers(MANIFEST_CACHE_CONTROL),
        Json(json!({
            "updateTime": freshness.update_time,
            "revision": freshness.revision,
            "catalogHash": freshness.hash,
            "counts": { "songs": song_count, "sheets": sheet_count, "charts": chart_count }
        })),
    )
        .into_response())
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
) -> Result<Json<Value>, AppError> {
    let last_full_reload: i64 = sqlx::query_scalar!(
        r#"SELECT last_full_reload_revision AS "v!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;

    if q.since < last_full_reload {
        return Err(AppError::SnapshotRequired);
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
    .await?;

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
        .await?;
        let sheet_rows = queries::fetch_sheets_for_song(&pool, song_pk).await?;
        let sheets = sheet_rows
            .into_iter()
            .map(|r| NestedSheet {
                sheet: r.into_meta(),
            })
            .collect();
        songs.push(Song {
            meta: song_row.into_meta(),
            sheets,
        });
    }

    let current_revision: i64 =
        sqlx::query_scalar!(r#"SELECT revision AS "v!" FROM catalog_meta LIMIT 1"#)
            .fetch_one(&pool)
            .await?;

    let deleted_song_ids: Vec<String> = sqlx::query_scalar!(
        r#"SELECT song_id AS "v!" FROM deleted_songs WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await?;
    let deleted_sheet_exprs: Vec<String> = sqlx::query_scalar!(
        r#"SELECT sheet_expr AS "v!" FROM deleted_sheets WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await?;

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
    use axum::extract::State as AxumState;
    use axum::http::{HeaderMap, StatusCode, header};

    async fn seed_catalog_meta(pool: &sqlx::PgPool) {
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(pool)
            .await
            .unwrap();
    }

    // The manifest shares /catalog's validator but not its Cache-Control:
    // its job is answering "did the catalog change", so it must always
    // revalidate. The 304 still has to repeat both (RFC 7232 §4.1).
    #[sqlx::test]
    async fn manifest_304_repeats_the_etag_and_revalidates(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_catalog_meta(&pool).await;

        let full = sync_manifest(HeaderMap::new(), AxumState(pool.clone()))
            .await
            .unwrap();
        let etag = full.headers().get(header::ETAG).unwrap().clone();
        assert_eq!(full.headers().get(header::CACHE_CONTROL).unwrap(), "no-cache");

        let mut conditional = HeaderMap::new();
        conditional.insert(header::IF_NONE_MATCH, etag.clone());

        let not_modified = sync_manifest(conditional, AxumState(pool)).await.unwrap();

        assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(not_modified.headers().get(header::ETAG), Some(&etag));
        assert_eq!(
            not_modified.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );
        Ok(())
    }

    // catalogHash in the body and the ETag are the same value, one quoted:
    // the contract (§1/§3) promises a client can compare them directly.
    #[sqlx::test]
    async fn manifest_catalog_hash_matches_the_unquoted_etag(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_catalog_meta(&pool).await;

        let response = sync_manifest(HeaderMap::new(), AxumState(pool)).await.unwrap();
        let etag = response.headers().get(header::ETAG).unwrap().to_str().unwrap().to_string();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(format!("\"{}\"", json["catalogHash"].as_str().unwrap()), etag);
        Ok(())
    }
}
