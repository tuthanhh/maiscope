use axum::{Json, Router, extract::State, routing::get};
use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::error::{AppError, PlainText};
use crate::queries;
use crate::routes::caching::snapshot;
use crate::routes::extract::{ApiPath, quoted};
use crate::state::AppState;
use crate::types::{OfficialSheet, OfficialSong};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/songs/{id}", get(song))
        .route("/songs/{id}/sheets/{type}/{difficulty}", get(sheet))
        .route("/songs/{id}/sheets/{type}/{difficulty}/chart", get(chart))
}

/// A song id from a path. Only the canonical form — lowercase, hyphenated, 36
/// characters — is an address, so a song has exactly one URL; `Uuid` would
/// also accept upper case, braces and `urn:uuid:` prefixes.
pub(crate) fn parse_song_id(raw: &str) -> Result<Uuid, AppError> {
    match Uuid::parse_str(raw) {
        Ok(id) if id.to_string() == raw => Ok(id),
        _ => Err(AppError::BadRequest(format!(
            "'{}' is not a song id (a lowercase hyphenated UUID)",
            quoted(raw)
        ))),
    }
}

/// The `404` for a sheet address, quoting each part capped.
fn sheet_not_found(kind: &'static str, id: &str, r#type: &str, difficulty: &str) -> AppError {
    AppError::NotFound {
        kind,
        key: format!("{}/{}/{}", quoted(id), quoted(r#type), quoted(difficulty)),
    }
}

// GET /api/songs/{id} — one song with its sheets, shaped as an element of the
// Catalog's `songs`: the song modal opened by a shared link, without the 3MB
// snapshot.
async fn song(
    ApiPath(id): ApiPath<String>,
    State(pool): State<PgPool>,
) -> Result<Json<OfficialSong>, AppError> {
    let song_id = parse_song_id(&id)?;
    let mut tx = snapshot(&pool).await?;
    let Some(row) = queries::fetch_official_songs(&mut tx, Some(song_id))
        .await?
        .pop()
    else {
        return Err(AppError::NotFound {
            kind: "song",
            key: quoted(&id),
        });
    };
    let sheets = queries::fetch_official_sheets(&mut tx, Some(song_id), None)
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
    ApiPath((id, r#type, difficulty)): ApiPath<(String, String, String)>,
    State(pool): State<PgPool>,
) -> Result<Json<OfficialSheet>, AppError> {
    let song_id = parse_song_id(&id)?;
    let mut tx = snapshot(&pool).await?;
    let Some(sheet) =
        queries::fetch_official_sheets(&mut tx, Some(song_id), Some((&r#type, &difficulty)))
            .await?
            .pop()
    else {
        return Err(sheet_not_found("sheet", &id, &r#type, &difficulty));
    };
    // `sheets.song_id_fk` is NOT NULL and references `songs`, and both reads
    // share one snapshot, so a found sheet has its song.
    let song = queries::fetch_official_songs(&mut tx, Some(song_id))
        .await?
        .pop()
        .ok_or(AppError::Internal("sheet without its song"))?;
    Ok(Json(OfficialSheet {
        fields: sheet.into(),
        song: song.into(),
    }))
}

// GET /api/songs/{id}/sheets/{type}/{difficulty}/chart — the simai text for
// the engine. Errors are plain text because the engine reads this body as
// text whatever the status; they are `AppError`s all the same, so a database
// failure is "internal error", never the sqlx message v1's handler sent. The
// path is taken as a `Result` so its rejection is rendered as text too.
async fn chart(
    path: Result<ApiPath<(String, String, String)>, AppError>,
    State(pool): State<PgPool>,
) -> Result<String, PlainText> {
    let ApiPath((id, r#type, difficulty)) = path?;
    let song_id = parse_song_id(&id)?;
    let mut conn = pool.acquire().await?;
    match queries::fetch_chart_text(&mut conn, song_id, &r#type, &difficulty).await? {
        queries::ChartText::Inline(text) => Ok(text),
        queries::ChartText::NoSheet => {
            Err(sheet_not_found("sheet", &id, &r#type, &difficulty).into())
        }
        queries::ChartText::NoChart => {
            Err(sheet_not_found("chart", &id, &r#type, &difficulty).into())
        }
        queries::ChartText::NotInline => Err(AppError::NotImplemented(format!(
            "chart for '{}/{}/{}' has no inline simai content",
            quoted(&id),
            quoted(&r#type),
            quoted(&difficulty)
        ))
        .into()),
    }
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
            ApiPath((id.to_string(), r#type.to_string(), code.to_string())),
            State(pool.clone()),
        )
        .await
        .map(value)
    }

    #[sqlx::test]
    async fn a_song_comes_with_its_sheets_as_in_the_catalog(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        let body = value(song(ApiPath(id.to_string()), State(pool)).await.unwrap());
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
        let error = song(ApiPath(unknown), State(pool.clone()))
            .await
            .unwrap_err();
        assert_eq!(status(error), StatusCode::NOT_FOUND);

        for bad in ["not-a-uuid", "upstream-key"] {
            let error = song(ApiPath(bad.to_string()), State(pool.clone()))
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

    async fn get_chart(pool: &PgPool, id: &str, code: &str) -> axum::response::Response {
        match chart(
            Ok(ApiPath((
                id.to_string(),
                "utage".to_string(),
                code.to_string(),
            ))),
            State(pool.clone()),
        )
        .await
        {
            Ok(text) => text.into_response(),
            Err(error) => error.into_response(),
        }
    }

    async fn text_of(response: axum::response::Response) -> String {
        String::from_utf8(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap()
    }

    /// Multi-line, CRLF, non-ASCII: the engine parses these exact bytes.
    const CHART: &str = "&title=協奏曲\r\n&inote_7=\r\n(150){4}1,2,\nE\n";

    /// Chart rows for the seeded UTAGE sheets: inline text on `協`, a
    /// blob-only row on `X`.
    async fn seed_charts(pool: &PgPool, song: Uuid) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash)
             SELECT id, sheet_expr, 'maimai-simai', $2, 'h'
             FROM sheets WHERE song_id_fk = $1 AND difficulty = '【協】'",
            song,
            CHART
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "INSERT INTO charts (sheet_id, sheet_expr, format)
             SELECT id, sheet_expr, 'maimai-simai'
             FROM sheets WHERE song_id_fk = $1 AND difficulty = '【X】'",
            song
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    #[sqlx::test]
    async fn the_chart_is_plain_text(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        seed_charts(&pool, id).await?;
        let response = get_chart(&pool, &id.to_string(), "協").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[axum::http::header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/plain")
        );
        assert_eq!(
            text_of(response).await,
            CHART,
            "the text is served byte for byte"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn chart_misses_are_plain_text_too(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        seed_charts(&pool, id).await?;
        let id = id.to_string();

        let blob_only = get_chart(&pool, &id, "X").await;
        assert_eq!(blob_only.status(), StatusCode::NOT_IMPLEMENTED);
        assert!(
            blob_only.headers()[axum::http::header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/plain")
        );
        assert_eq!(
            text_of(blob_only).await,
            format!("chart for '{id}/utage/X' has no inline simai content")
        );

        // The master sheet exists and has no chart row; `即` is no sheet.
        let no_chart = chart(
            Ok(ApiPath((
                id.clone(),
                "dx".to_string(),
                "master".to_string(),
            ))),
            State(pool.clone()),
        )
        .await
        .unwrap_err()
        .into_response();
        assert_eq!(no_chart.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            text_of(no_chart).await,
            format!("chart '{id}/dx/master' not found")
        );
        let no_sheet = get_chart(&pool, &id, "即").await;
        assert_eq!(no_sheet.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            text_of(no_sheet).await,
            format!("sheet '{id}/utage/即' not found")
        );

        let bad_id = get_chart(&pool, "nope", "協").await;
        assert_eq!(bad_id.status(), StatusCode::BAD_REQUEST);
        assert!(text_of(bad_id).await.starts_with("'nope' is not a song id"));
        Ok(())
    }

    // The leak v1's handler had: a database error's own text in the body.
    #[sqlx::test]
    async fn a_database_error_never_reaches_the_body(pool: PgPool) -> sqlx::Result<()> {
        let id = seed(&pool).await?;
        sqlx::query("ALTER TABLE charts RENAME TO charts_gone")
            .execute(&pool)
            .await?;
        let response = get_chart(&pool, &id.to_string(), "協").await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(text_of(response).await, "internal error");
        Ok(())
    }

    // The chart route through the router, encoded as a browser sends it.
    #[sqlx::test]
    async fn the_chart_route_is_mounted_and_decodes_its_code(pool: PgPool) -> sqlx::Result<()> {
        use tower::ServiceExt;
        let id = seed(&pool).await?;
        seed_charts(&pool, id).await?;
        let app = router().with_state(test_support::state(pool, test_support::config()));
        let request = axum::http::Request::builder()
            .uri(format!("/songs/{id}/sheets/utage/%E5%8D%94/chart"))
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(text_of(response).await, CHART);
        Ok(())
    }
}
