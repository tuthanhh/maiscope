use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;

use crate::error::AppError;
use crate::queries;
use crate::routes::caching::{CATALOG_CACHE_CONTROL, Freshness, snapshot};
use crate::state::AppState;
use crate::types::{OfficialCatalog, OfficialSheetFields, OfficialSong};

pub fn router() -> Router<AppState> {
    Router::new().route("/catalog", get(catalog))
}

// GET /api/catalog — the whole official library, nested songs[].sheets[], with
// the reference tables beside it (ADR-0021). The client builds its flat sheet
// list with an explicit `sheet.song`; the wire stays nested because flattening
// repeats every song field per sheet.
async fn catalog(headers: HeaderMap, State(pool): State<PgPool>) -> Result<Response, AppError> {
    // ETag, songs, sheets and reference tables from one snapshot: a sync
    // committing midway would otherwise cache a mixed Catalog for an hour.
    let mut tx = snapshot(&pool).await?;
    let freshness = Freshness::load(&mut *tx).await?;
    if freshness.is_current_for(&headers) {
        return Ok(freshness.not_modified(CATALOG_CACHE_CONTROL));
    }

    let mut sheets_by_song: HashMap<_, Vec<OfficialSheetFields>> = HashMap::new();
    for row in queries::fetch_official_sheets(&mut tx, None, None).await? {
        sheets_by_song
            .entry(row.song_id)
            .or_default()
            .push(row.into());
    }
    let songs = queries::fetch_official_songs(&mut tx, None)
        .await?
        .into_iter()
        .map(|row| OfficialSong {
            sheets: sheets_by_song.remove(&row.id).unwrap_or_default(),
            fields: row.into(),
        })
        .collect();

    let catalog = OfficialCatalog {
        songs,
        reference: queries::fetch_reference(&mut tx).await?,
    };
    Ok((freshness.headers(CATALOG_CACHE_CONTROL), Json(catalog)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{StatusCode, header};
    use serde_json::{Value, json};

    use crate::test_support;

    /// One song with a standard sheet and a UTAGE sheet, every optional field
    /// filled, and a chart for the first. Returns the song's id.
    async fn seed(pool: &PgPool) -> sqlx::Result<sqlx::types::Uuid> {
        test_support::seed_difficulties(pool, &["master"]).await;
        sqlx::query!(
            "INSERT INTO difficulties (difficulty, name, code, ordinal)
             VALUES ('【協】', '【協】', '協', 1)"
        )
        .execute(pool)
        .await?;
        let song = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, category, title, artist, bpm, image_name, version,
                                release_date, is_new, is_locked, comment, source_index)
             VALUES ('upstream-key', 'POPS', 'Title', 'Artist', 150, 'img.png', 'MAGiCAL',
                     '2024-01-02', true, false, 'note', 0)
             RETURNING id"
        )
        .fetch_one(pool)
        .await?;
        let master = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level, level_value,
                                 internal_level, internal_level_value, note_designer,
                                 is_special, source_index)
             VALUES ($1, 'upstream-key|dx|master', 'dx', 'master', '13+', 13.7, '13.8', 13.8,
                     'designer', true, 0)
             RETURNING id",
            song
        )
        .fetch_one(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, 'upstream-key|utage|【協】', 'utage', '【協】', 1)",
            song
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheet_note_counts (sheet_id, key, value) VALUES ($1, 'total', 900)",
            master
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheet_regions (sheet_id, region, available) VALUES ($1, 'jp', true)",
            master
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheet_region_overrides
                 (sheet_id, region, level, level_value, internal_level,
                  internal_level_value, note_designer)
             VALUES ($1, 'intl', '13', 13.0, '13.2', 13.2, 'intl designer')",
            master
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash)
             VALUES ($1, 'upstream-key|dx|master', 'maimai-simai', '&title=x', 'h')",
            master
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal)
             VALUES ('MAGiCAL', 'MAGiCAL', '2024-01-01', 0)"
        )
        .execute(pool)
        .await?;
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(pool)
            .await?;
        Ok(song)
    }

    async fn body(response: Response) -> Value {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    // The mapping check the spec's tripwire asks for: every key the payload
    // carries is one ADR-0026 lists, each holding its column's value — not a
    // comparison with v1's bytes.
    #[sqlx::test]
    async fn the_catalog_maps_each_column_to_its_v2_key(pool: PgPool) -> sqlx::Result<()> {
        let song = seed(&pool).await?;
        let response = catalog(HeaderMap::new(), State(pool)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let catalog = body(response).await;

        assert_eq!(
            catalog["songs"],
            json!([{
                "id": song.to_string(),
                "category": "POPS",
                "title": "Title",
                "artist": "Artist",
                "bpm": 150.0,
                "imageName": "img.png",
                "version": "MAGiCAL",
                "releaseDate": "2024-01-02",
                "isLocked": false,
                "comment": "note",
                "sheets": [
                    {
                        "type": "dx",
                        "difficulty": "master",
                        "level": "13+",
                        "levelValue": 13.7,
                        "internalLevel": "13.8",
                        "internalLevelValue": 13.8,
                        "noteDesigner": "designer",
                        "noteCounts": { "total": 900 },
                        "regions": { "jp": true },
                        // All five keys, so a swapped pair in the SQL fails.
                        "regionOverrides": { "intl": {
                            "level": "13", "levelValue": 13.0,
                            "internalLevel": "13.2", "internalLevelValue": 13.2,
                            "noteDesigner": "intl designer"
                        } },
                        "hasChart": true
                    },
                    // A UTAGE sheet carries its code, not its bracketed name.
                    { "type": "utage", "difficulty": "協", "hasChart": false }
                ]
            }])
        );
        // The reference tables ride beside the songs.
        let mut keys: Vec<&str> = catalog
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "categories",
                "difficulties",
                "regions",
                "songs",
                "types",
                "versions"
            ]
        );
        Ok(())
    }

    /// Every key in the document, at any depth.
    fn keys(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    out.push(key.clone());
                    keys(value, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| keys(item, out)),
            _ => {}
        }
    }

    #[sqlx::test]
    async fn no_derived_or_dropped_field_appears_anywhere(pool: PgPool) -> sqlx::Result<()> {
        seed(&pool).await?;
        let catalog = body(catalog(HeaderMap::new(), State(pool)).await.unwrap()).await;
        let mut found = Vec::new();
        keys(&catalog, &mut found);
        for forbidden in [
            // Derived on the client.
            "songNo",
            "imageUrl",
            "imageUrlM",
            "sheetExpr",
            "notePercents",
            "$canonicalSheet",
            // Dropped by ADR-0026.
            "songId",
            "isNew",
            "isSpecial",
            "updateTime",
            "iconUrl",
            "iconHeight",
        ] {
            assert!(
                !found.iter().any(|k| k == forbidden),
                "{forbidden} is on the wire"
            );
        }
        // `releaseDate` is a song field, so it is checked where it is dropped.
        assert!(!catalog["versions"].as_array().unwrap().is_empty());
        for version in catalog["versions"].as_array().unwrap() {
            assert!(version.get("releaseDate").is_none(), "{version}");
        }
        Ok(())
    }

    #[sqlx::test]
    async fn the_catalog_answers_304_while_unchanged(pool: PgPool) -> sqlx::Result<()> {
        seed(&pool).await?;
        let first = catalog(HeaderMap::new(), State(pool.clone()))
            .await
            .unwrap();
        assert_eq!(
            first.headers()[header::CACHE_CONTROL],
            CATALOG_CACHE_CONTROL
        );
        let mut conditional = HeaderMap::new();
        conditional.insert(header::IF_NONE_MATCH, first.headers()[header::ETAG].clone());
        let second = catalog(conditional, State(pool)).await.unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        // RFC 7232 §4.1: the 304 repeats the validators the 200 sent.
        assert_eq!(
            second.headers()[header::ETAG],
            first.headers()[header::ETAG]
        );
        assert_eq!(
            second.headers()[header::CACHE_CONTROL],
            CATALOG_CACHE_CONTROL
        );
        Ok(())
    }

    // A database between `migrate` and the first sync — every new
    // environment's first state — answers an empty Catalog, not a 500
    // (`prod-data-and-infra` 06, once v1's test; now this one).
    #[sqlx::test]
    async fn an_unsynced_database_answers_an_empty_catalog(pool: PgPool) {
        let response = catalog(HeaderMap::new(), State(pool)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(header::ETAG));
        let catalog = body(response).await;
        assert_eq!(catalog["songs"], json!([]));
        for table in ["categories", "versions", "types", "difficulties", "regions"] {
            assert!(catalog[table].is_array(), "{table}");
        }
    }
}
