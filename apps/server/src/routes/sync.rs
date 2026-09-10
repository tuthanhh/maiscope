use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::routes::catalog::catalog_hash;
use crate::state::AppState;
use crate::types::{self, NestedSheet, Song};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync/manifest", get(sync_manifest))
        .route("/sync/delta", get(sync_delta))
}

// GET /sync/manifest — cheap freshness probe (contract §3).
async fn sync_manifest(State(pool): State<Pool<Postgres>>) -> Result<Json<Value>, AppError> {
    let row = sqlx::query!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
           FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;

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
