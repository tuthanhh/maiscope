use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::domain::{ChartType, Difficulty};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/sheets/{sheet}/chart", get(get_chart))
}

// Query params for GET /sheets/{sheet}/chart, e.g. ?type=dx&difficulty=master.
// type is required: a song has separate dx/std sheets at the same difficulty.
#[derive(Debug, Deserialize)]
struct ChartQuery {
    r#type: ChartType,
    difficulty: Difficulty,
}

// GET /sheets/{songId}/chart?type=dx&difficulty=master
// Returns raw simai chart text (text/plain) for the wasm engine to parse.
//
// This is the one handler that does not return `AppError`. That is deliberate,
// not an oversight: contract §2 specifies plain-text errors here, and §6 lists
// the endpoint as the documented exception to the JSON error envelope. The
// success path is `text/plain` because the engine wants the simai itself, so an
// error path returning JSON would be the less consistent design. Converting it
// would change the wire format of a shipped endpoint that `useEngine.ts`
// consumes as text.
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

    /// Insert a sheet, and optionally a chart row for it. `content: None` is the
    /// blob-only case — a chart row that exists but holds no inline simai.
    async fn seed_sheet(pool: &sqlx::PgPool, sheet_expr: &str, content: Option<&str>) {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index)
             VALUES ('example', 'Example Song', 0)
             ON CONFLICT (song_id) DO NOTHING"
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = 'example'",
            sheet_expr
        )
        .execute(pool)
        .await
        .unwrap();

        if let Some(text) = content {
            sqlx::query!(
                "INSERT INTO charts (sheet_id, sheet_expr, content, format)
                 SELECT id, $1, $2, 'maimai-simai' FROM sheets WHERE sheet_expr = $1",
                sheet_expr,
                text
            )
            .execute(pool)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "INSERT INTO charts (sheet_id, sheet_expr, format)
                 SELECT id, $1, 'maimai-simai' FROM sheets WHERE sheet_expr = $1",
                sheet_expr
            )
            .execute(pool)
            .await
            .unwrap();
        }
    }

    fn query() -> AxumQuery<ChartQuery> {
        AxumQuery(ChartQuery {
            r#type: ChartType::Dx,
            difficulty: Difficulty::Master,
        })
    }

    /// The chart text round-trips byte-identical. The engine parses whatever
    /// comes back, so any normalisation here — trimming, re-encoding newlines —
    /// would silently change how a chart plays.
    #[sqlx::test]
    async fn get_chart_returns_the_simai_verbatim(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let simai = "(120){8}\n1,2,3/5,\n1-5[8:1],\nE\n";
        seed_sheet(&pool, "example|dx|master", Some(simai)).await;

        let body = get_chart(Path("example".to_string()), query(), AxumState(pool))
            .await
            .unwrap();

        assert_eq!(body, simai);
        Ok(())
    }

    /// The path segment is the songId alone; the server rebuilds sheet_expr from
    /// the three parts (contract §2). A wrong difficulty must not fall back to
    /// whatever chart the song does have.
    #[sqlx::test]
    async fn get_chart_rebuilds_the_sheet_expr_from_all_three_parts(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        seed_sheet(&pool, "example|dx|master", Some("1,2,E")).await;

        let wrong_difficulty = get_chart(
            Path("example".to_string()),
            AxumQuery(ChartQuery {
                r#type: ChartType::Dx,
                difficulty: Difficulty::Expert,
            }),
            AxumState(pool),
        )
        .await;

        let (status, message) = wrong_difficulty.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(message.contains("example|dx|expert"), "{message}");
        Ok(())
    }

    #[sqlx::test]
    async fn get_chart_404s_when_no_chart_row_exists(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let (status, message) = get_chart(Path("nope".to_string()), query(), AxumState(pool))
            .await
            .unwrap_err();

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(message.contains("nope|dx|master"), "{message}");
        Ok(())
    }

    /// A chart row with no inline content is `501`, not `404` — the difference
    /// between "we do not have this" and "we have it in a form we cannot serve
    /// yet" (contract §2).
    #[sqlx::test]
    async fn get_chart_501s_for_a_row_without_inline_content(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        seed_sheet(&pool, "example|dx|master", None).await;

        let (status, message) = get_chart(Path("example".to_string()), query(), AxumState(pool))
            .await
            .unwrap_err();

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert!(message.contains("no inline simai content"), "{message}");
        Ok(())
    }
}
