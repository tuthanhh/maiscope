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

    /// Fields the server must never emit. `preprocessData` derives all of them
    /// client-side (`utils/data.ts`), and the split is a contract convention —
    /// the server sends raw fields only.
    const DERIVED_FIELDS: [&str; 6] = [
        "songNo",
        "imageUrl",
        "imageUrlM",
        "sheetExpr",
        "notePercents",
        "$canonicalSheet",
    ];

    /// Every key in the document, at any depth, with the path that reached it.
    fn walk_keys(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    out.push((key.clone(), path.to_string()));
                    walk_keys(child, &format!("{path}.{key}"), out);
                }
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    walk_keys(child, &format!("{path}[{i}]"), out);
                }
            }
            _ => {}
        }
    }

    /// A catalog with one song, two sheets and every lookup table populated —
    /// enough that the snapshot covers each branch of the response assembly
    /// rather than only the happy path of a bare song.
    ///
    /// `update_time` is fixed, not `now()`: the snapshot has to be byte-stable
    /// across runs.
    async fn seed_snapshot_catalog(pool: &sqlx::PgPool) {
        // A literal rather than a bound parameter: binding a timestamp needs a
        // chrono/time type, and `server` depends on neither directly.
        sqlx::query!(
            "INSERT INTO catalog_meta (id, update_time, revision)
             VALUES (true, TIMESTAMPTZ '2026-01-02T03:04:05Z', 7)"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ('POPS & ANIME', 0)")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal)
             VALUES ('PRiSM', 'PRiSM', DATE '2025-09-11', 0)"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO types (type, name, abbr, icon_url, icon_height, ordinal)
             VALUES ('dx', 'DX', 'DX', 'dx.png', 32, 0)"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO difficulties (difficulty, name, color, icon_url, icon_height, ordinal)
             VALUES ('master', 'MASTER', '#9f51dc', 'master.png', 20, 0)"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO regions (region, name, ordinal) VALUES ('jp', 'Japan', 0)")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query!(
            "INSERT INTO songs (song_id, category, title, artist, bpm, image_name, version,
                                release_date, is_new, is_locked, comment, source_index)
             VALUES ('example', 'POPS & ANIME', 'Example Song', 'Example Artist', 174,
                     'example.png', 'PRiSM', DATE '2025-09-11', true, false, 'a comment', 0)"
        )
        .execute(pool)
        .await
        .unwrap();

        // A fully-populated sheet, and a sparse one — the optional fields are
        // where a serde rename or a skip_serializing_if would go unnoticed.
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level, level_value,
                                 internal_level, internal_level_value, note_designer, is_special,
                                 source_index)
             SELECT id, 'example|dx|master', 'dx', 'master', '14+', 14.5, '14.7', 14.7,
                    'Example Designer', false, 0
             FROM songs WHERE song_id = 'example'"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, 'example|std|basic', 'std', 'basic', 1
             FROM songs WHERE song_id = 'example'"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO sheet_note_counts (sheet_id, key, value)
             SELECT id, k.key, k.value
             FROM sheets, (VALUES ('total', 1000), ('tap', 500), ('hold', 100),
                                  ('slide', 200), ('touch', 150), ('break', 50))
                          AS k(key, value)
             WHERE sheet_expr = 'example|dx|master'"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO sheet_regions (sheet_id, region, available)
             SELECT id, 'jp', true FROM sheets WHERE sheet_expr = 'example|dx|master'"
        )
        .execute(pool)
        .await
        .unwrap();
    }

    /// The response contract, pinned byte-for-byte.
    ///
    /// The frontend rebuilds a prototype-linked object graph from this payload
    /// in `utils/data.ts:preprocessData`, so a dropped or renamed field is not a
    /// compile error anywhere — it surfaces as a blank column or a crash deep in
    /// the client. This is the cheap substitute for wiring the `shared` crate,
    /// which would not help anyway: generated types would describe what the
    /// server sends, and the risk is that what it sends stops matching what the
    /// client reads.
    ///
    /// Regenerate deliberately:
    /// `UPDATE_SNAPSHOT=1 cargo test -p server catalog_response_matches_snapshot`
    /// — then read the diff. A snapshot refreshed without reading is a snapshot
    /// that asserts nothing.
    #[sqlx::test]
    async fn catalog_response_matches_snapshot(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_snapshot_catalog(&pool).await;

        let response = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool),
        )
        .await
        .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let actual = format!("{}\n", serde_json::to_string_pretty(&json).unwrap());

        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/catalog.json");

        if std::env::var_os("UPDATE_SNAPSHOT").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &actual).unwrap();
            return Ok(());
        }

        let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "{} is missing — create it with \
                 UPDATE_SNAPSHOT=1 cargo test -p server catalog_response_matches_snapshot",
                path.display()
            )
        });

        assert_eq!(
            expected, actual,
            "\n/catalog's response shape changed. If that is intended:\n  \
             UPDATE_SNAPSHOT=1 cargo test -p server catalog_response_matches_snapshot\n"
        );
        Ok(())
    }

    /// The server must never emit a field the client derives.
    ///
    /// Separate from the snapshot on purpose. The snapshot pins what *is* sent,
    /// which catches an addition only if someone reads the diff; this asserts
    /// the absence directly, at any nesting depth, so a derived field appearing
    /// inside `songs[].sheets[]` fails by name rather than as one line in a
    /// large diff.
    #[sqlx::test]
    async fn catalog_never_emits_derived_fields(pool: sqlx::PgPool) -> sqlx::Result<()> {
        seed_snapshot_catalog(&pool).await;

        let response = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool),
        )
        .await
        .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        let mut keys = Vec::new();
        walk_keys(&json, "$", &mut keys);
        assert!(!keys.is_empty(), "walked an empty document");

        for (key, path) in &keys {
            assert!(
                !DERIVED_FIELDS.contains(&key.as_str()),
                "server emitted the client-derived field {key:?} at {path} — \
                 see api-contract.md's raw-versus-derived rule"
            );
        }

        // The walk reaches nested sheets, or the assertion above proves nothing.
        assert!(
            keys.iter()
                .any(|(k, p)| k == "type" && p.contains("sheets")),
            "walk never descended into songs[].sheets[]"
        );
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
    async fn catalog_returns_304_when_if_none_match_matches(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
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
    async fn catalog_304_repeats_the_etag_and_cache_control(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        seed_minimal_catalog(&pool).await;

        let full = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool.clone()),
        )
        .await
        .unwrap();
        let etag = full
            .headers()
            .get(axum::http::header::ETAG)
            .unwrap()
            .clone();
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
            not_modified
                .headers()
                .get(axum::http::header::CACHE_CONTROL),
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
    async fn catalog_etag_changes_when_catalog_meta_changes(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
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

    // No seeding at all: migrations have run, every table is empty. This is
    // the state every freshly provisioned environment starts in (contract §1,
    // docs/work/prod-data-and-infra/issues/06-empty-catalog-500.md).
    #[sqlx::test]
    async fn catalog_on_an_unseeded_database_is_empty_not_an_error(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        let response = catalog(
            AxumQuery(CatalogQuery { region: None }),
            axum::http::HeaderMap::new(),
            AxumState(pool),
        )
        .await
        .expect("an unseeded catalog must not be an error");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["updateTime"], "0000-00-00");
        assert_eq!(json["songs"].as_array().unwrap().len(), 0);
        assert_eq!(json["categories"].as_array().unwrap().len(), 0);
        Ok(())
    }
}
