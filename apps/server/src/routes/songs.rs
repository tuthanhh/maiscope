use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::{Pool, Postgres};

use crate::error::AppError;
use crate::queries;
use crate::state::AppState;
use crate::types::{NestedSheet, Song};

pub fn router() -> Router<AppState> {
    Router::new().route("/songs/{id}", get(get_song))
}

// GET /songs/{id} — single Song with its sheets (types/Song.ts).
async fn get_song(
    Path(id): Path<String>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Song>, AppError> {
    let Some(song_row) = queries::fetch_song_by_song_id(&pool, &id).await? else {
        return Err(AppError::NotFound {
            kind: "song",
            key: id,
        });
    };

    let sheet_rows = queries::fetch_sheets_for_song(&pool, song_row.id).await?;

    let sheets = sheet_rows
        .into_iter()
        .map(|row| NestedSheet {
            sheet: row.into_meta(),
        })
        .collect();

    Ok(Json(Song {
        meta: song_row.into_meta(),
        sheets,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State as AxumState;

    #[sqlx::test]
    async fn get_song_returns_404_for_unknown_id(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let result = get_song(Path("nope".to_string()), AxumState(pool)).await;
        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn get_song_returns_song_with_sheets(pool: sqlx::PgPool) -> sqlx::Result<()> {
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

        let result = get_song(Path("maimai_song".to_string()), AxumState(pool))
            .await
            .unwrap();

        assert_eq!(result.0.sheets.len(), 1);
        Ok(())
    }
}
