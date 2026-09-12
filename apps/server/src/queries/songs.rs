use sqlx::PgPool;

pub async fn fetch_all_songs(pool: &PgPool) -> Result<Vec<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs ORDER BY source_index"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_song_by_song_id(
    pool: &PgPool,
    song_id: &str,
) -> Result<Option<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs WHERE song_id = $1"#,
        song_id
    )
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn fetch_song_by_song_id_returns_none_when_missing(pool: PgPool) -> sqlx::Result<()> {
        let result = fetch_song_by_song_id(&pool, "does_not_exist")
            .await
            .unwrap();
        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_song_by_song_id_finds_row(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_song_by_song_id(&pool, "maimai_song").await.unwrap();

        assert_eq!(
            result.map(|r| r.title),
            Some(Some("Example Song".to_string()))
        );
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_all_songs_orders_by_source_index(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "second",
            "Second",
            1
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "first",
            "First",
            0
        )
        .execute(&pool)
        .await?;

        let songs = fetch_all_songs(&pool).await.unwrap();

        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title.as_deref(), Some("First"));
        assert_eq!(songs[1].title.as_deref(), Some("Second"));
        Ok(())
    }
}
