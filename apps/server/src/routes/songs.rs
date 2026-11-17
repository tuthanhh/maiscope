use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types::{OfficialSheet, OfficialSong};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/songs/{id}", get(song))
        .route("/songs/{id}/sheets/{type}/{difficulty}", get(sheet))
}

/// A song id from a path. Taken as a `String` and parsed here, so a malformed
/// one is a `400` in the contract's JSON shape rather than axum's plain-text
/// path rejection.
pub(crate) fn parse_song_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest(format!("'{raw}' is not a song id")))
}

// GET /api/songs/{id} — one song with its sheets, shaped as an element of the
// Catalog's `songs`: the song modal opened by a shared link, without the 3MB
// snapshot.
async fn song(
    Path(id): Path<String>,
    State(pool): State<PgPool>,
) -> Result<Json<OfficialSong>, AppError> {
    let song_id = parse_song_id(&id)?;
    let Some(row) = queries::fetch_official_songs(&pool, Some(song_id))
        .await?
        .pop()
    else {
        return Err(AppError::NotFound {
            kind: "song",
            key: id,
        });
    };
    let sheets = queries::fetch_official_sheets(&pool, Some(song_id), None)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(Json(OfficialSong {
        fields: row.into(),
        sheets,
    }))
}

// GET /api/songs/{id}/sheets/{type}/{difficulty} — one sheet and its song: the
// visualizer's metadata card. `{difficulty}` is `difficulties.code`, arriving
// percent-decoded (`%E5%8D%94` → `協`). An unknown type or code is simply a
// sheet that does not exist: the lookup is the validation.
async fn sheet(
    Path((id, r#type, difficulty)): Path<(String, String, String)>,
    State(pool): State<PgPool>,
) -> Result<Json<OfficialSheet>, AppError> {
    let song_id = parse_song_id(&id)?;
    let not_found = || AppError::NotFound {
        kind: "sheet",
        key: format!("{id}/{type}/{difficulty}"),
    };
    let Some(sheet) =
        queries::fetch_official_sheets(&pool, Some(song_id), Some((&r#type, &difficulty)))
            .await?
            .pop()
    else {
        return Err(not_found());
    };
    let Some(song) = queries::fetch_official_songs(&pool, Some(song_id))
        .await?
        .pop()
    else {
        return Err(not_found());
    };
    Ok(Json(OfficialSheet {
        fields: sheet.into(),
        song: song.into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use serde_json::{Value, json};

    use crate::test_support;

    async fn seed(pool: &PgPool) -> sqlx::Result<Uuid> {
        test_support::seed_difficulties(pool, &["master"]).await;
        sqlx::query!(
            "INSERT INTO difficulties (difficulty, name, code, ordinal)
             VALUES ('【協】', '【協】', '協', 1), ('【X】', '【X】', 'X', 2)"
        )
        .execute(pool)
        .await?;
        let song = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, title, artist, source_index)
             VALUES ('upstream-key', 'Title', 'Artist', 0) RETURNING id"
        )
        .fetch_one(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level, source_index)
             VALUES ($1, 'upstream-key|dx|master', 'dx', 'master', '13+', 0),
                    ($1, 'upstream-key|utage|【協】', 'utage', '【協】', '14?', 1),
                    ($1, 'upstream-key|utage|【X】', 'utage', '【X】', '13?', 2)",
            song
        )
        .execute(pool)
        .await?;
        Ok(song)
    }

    fn value<T: serde::Serialize>(Json(body): Json<T>) -> Value {
        serde_json::to_value(body).unwrap()
    }

    fn status(error: AppError) -> StatusCode {
        error.into_response().status()
    }

    async fn get_sheet(
        pool: &PgPool,
        id: &str,
        r#type: &str,
        code: &str,
    ) -> Result<Value, AppError> {
        sheet(
            Path((id.to_string(), r#type.to_string(), code.to_string())),
            State(pool.clone()),
        )
        .await
        .map(value)
    }

    #[sqlx::test]
    async fn a_song_comes_with_its_sheets_as_in_the_catalog(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        let body = value(song(Path(id.to_string()), State(pool)).await.unwrap());
        assert_eq!(body["id"], id.to_string());
        assert_eq!(body["title"], "Title");
        let codes: Vec<&str> = body["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["difficulty"].as_str().unwrap())
            .collect();
        assert_eq!(codes, ["master", "協", "X"]);
        Ok(())
    }

    #[sqlx::test]
    async fn a_sheet_carries_its_song_as_a_reference(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        let body = get_sheet(&pool, &id.to_string(), "dx", "master")
            .await
            .unwrap();
        assert_eq!(
            body,
            json!({
                "type": "dx", "difficulty": "master", "level": "13+", "hasChart": false,
                "song": { "id": id.to_string(), "title": "Title", "artist": "Artist" }
            })
        );
        Ok(())
    }

    #[sqlx::test]
    async fn a_utage_sheet_is_found_by_its_code(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?.to_string();
        // Axum hands the handler the decoded segment; `/utage/%E5%8D%94` is `協`.
        let body = get_sheet(&pool, &id, "utage", "協").await.unwrap();
        assert_eq!(body["difficulty"], "協");
        assert_eq!(body["level"], "14?");
        // The bracketed name is not an address, and ASCII codes keep their case.
        for (code, why) in [("【協】", "the name"), ("x", "a lower-cased code")] {
            let error = get_sheet(&pool, &id, "utage", code).await.unwrap_err();
            assert_eq!(status(error), StatusCode::NOT_FOUND, "{why} found a sheet");
        }
        assert_eq!(
            get_sheet(&pool, &id, "utage", "X").await.unwrap()["level"],
            "13?"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn misses_are_404_and_a_malformed_id_is_400(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?.to_string();
        // Well-formed, never minted.
        let unknown = "01a1e99a-0000-7000-8000-000000000000".to_string();
        for (song_id, r#type, code) in [
            (unknown.as_str(), "dx", "master"),
            (id.as_str(), "std", "master"),
            (id.as_str(), "dx", "expert"),
            (id.as_str(), "nonsense", "master"),
        ] {
            let error = get_sheet(&pool, song_id, r#type, code).await.unwrap_err();
            assert_eq!(
                status(error),
                StatusCode::NOT_FOUND,
                "{song_id}/{type}/{code}"
            );
        }
        let error = song(Path(unknown), State(pool.clone())).await.unwrap_err();
        assert_eq!(status(error), StatusCode::NOT_FOUND);

        for bad in ["not-a-uuid", "upstream-key"] {
            let error = song(Path(bad.to_string()), State(pool.clone()))
                .await
                .unwrap_err();
            assert_eq!(status(error), StatusCode::BAD_REQUEST, "{bad}");
        }
        Ok(())
    }

    // The handler tests pass the code already decoded; this one sends the
    // percent-encoded path a browser would, through the router.
    #[sqlx::test]
    async fn the_router_decodes_a_percent_encoded_code(pool: PgPool) -> sqlx::Result<()> {
        use tower::ServiceExt;
        let id = seed(&pool).await?;
        let app = router().with_state(test_support::state(pool, test_support::config()));
        let request = axum::http::Request::builder()
            .uri(format!("/songs/{id}/sheets/utage/%E5%8D%94"))
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["difficulty"], "協");
        Ok(())
    }
}
