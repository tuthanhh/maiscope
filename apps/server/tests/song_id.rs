//! `songs.id` is a minted UUID v7 and the identity every URL will carry
//! (`song-public-id`). These tests pin the properties that make it safe to
//! hand out: it is minted, it survives re-syncs, and the migration that
//! introduced it reverses without detaching sheets or chart text.

use server::catalog_sync::apply;
use server::upstream::RawData;
use sqlx::PgPool;
use sqlx::types::Uuid;

fn payload(json: &str) -> RawData {
    serde_json::from_str(json).unwrap()
}

const TWO_SONGS: &str = r#"{
    "songs": [
        { "songId": "Halcyon", "title": "Halcyon", "artist": "xi",
          "sheets": [{ "type": "dx", "difficulty": "master", "level": "13" }] },
        { "songId": "[協]Love You", "title": "[協]Love You",
          "sheets": [{ "type": "utage", "difficulty": "【協】", "level": "12?" }] }
    ]
}"#;

async fn ids_by_song_id(pool: &PgPool) -> Vec<(String, Uuid)> {
    sqlx::query_as("SELECT song_id, id FROM songs ORDER BY song_id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn every_synced_song_is_minted_a_uuid_v7(pool: PgPool) -> sqlx::Result<()> {
    apply(&pool, &payload(TWO_SONGS)).await?;

    let ids = ids_by_song_id(&pool).await;
    assert_eq!(ids.len(), 2);
    for (song_id, id) in &ids {
        assert_eq!(id.get_version_num(), 7, "{song_id} got {id}");
    }
    assert_ne!(ids[0].1, ids[1].1);
    Ok(())
}

// The primary key is identity-bearing: re-presenting a song the table
// already holds must reuse its id by matching on `song_id`, never mint a
// fresh one, or every URL for that song changes.
#[sqlx::test]
async fn re_presenting_a_song_reuses_its_id(pool: PgPool) -> sqlx::Result<()> {
    apply(&pool, &payload(TWO_SONGS)).await?;
    let before = ids_by_song_id(&pool).await;

    let edited = TWO_SONGS.replace(r#""artist": "xi""#, r#""artist": "xi (edited)""#);
    let stats = apply(&pool, &payload(&edited)).await?;

    assert_eq!(stats.songs_updated, 1, "the edit must actually be applied");
    assert_eq!(ids_by_song_id(&pool).await, before);
    Ok(())
}

// The regression that would break every link at once: the daily sync of an
// unchanged catalog must not touch a single id.
#[sqlx::test]
async fn an_unchanged_resync_keeps_every_id(pool: PgPool) -> sqlx::Result<()> {
    apply(&pool, &payload(TWO_SONGS)).await?;
    let before = ids_by_song_id(&pool).await;

    apply(&pool, &payload(TWO_SONGS)).await?;

    assert_eq!(ids_by_song_id(&pool).await, before);
    Ok(())
}

// A song upstream stops listing is tombstoned, not deleted, and keeps its
// id: a manual rename fix needs something to re-point, and a song that
// comes back must come back at the same URL.
#[sqlx::test]
async fn a_tombstoned_song_keeps_its_id_and_returns_with_it(pool: PgPool) -> sqlx::Result<()> {
    apply(&pool, &payload(TWO_SONGS)).await?;
    let before = ids_by_song_id(&pool).await;

    let halcyon_only = r#"{ "songs": [
        { "songId": "Halcyon", "title": "Halcyon", "artist": "xi",
          "sheets": [{ "type": "dx", "difficulty": "master", "level": "13" }] } ] }"#;
    let stats = apply(&pool, &payload(halcyon_only)).await?;
    assert_eq!(stats.songs_vanished, 1);
    assert_eq!(ids_by_song_id(&pool).await, before);

    apply(&pool, &payload(TWO_SONGS)).await?;
    assert_eq!(ids_by_song_id(&pool).await, before);
    Ok(())
}

const SONG_ID_MIGRATION: i64 = 20261117110000;

// The one change in this feature that is painful to undo by hand, because
// it moves a primary key. Down then up must leave every sheet on its song
// and every chart on its sheet.
#[sqlx::test]
async fn the_song_id_migration_reverses_with_sheets_and_charts_attached(
    pool: PgPool,
) -> sqlx::Result<()> {
    apply(&pool, &payload(TWO_SONGS)).await?;
    sqlx::query(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash) \
         SELECT id, sheet_expr, 'maimai-simai', 'E', 'h' FROM sheets",
    )
    .execute(&pool)
    .await?;

    let migrator = sqlx::migrate!("./migrations");
    migrator
        .undo(&pool, SONG_ID_MIGRATION - 1)
        .await
        .expect("down migration");
    let key_type: String = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns \
         WHERE table_name = 'songs' AND column_name = 'id'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(key_type, "bigint");
    assert_eq!(attached(&pool).await, 2);

    migrator.run(&pool).await.expect("up again");
    assert_eq!(attached(&pool).await, 2);
    for (_, id) in ids_by_song_id(&pool).await {
        assert_eq!(id.get_version_num(), 7);
    }
    Ok(())
}

/// Charts reachable from a song through their sheet.
async fn attached(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM charts c \
         JOIN sheets sh ON sh.id = c.sheet_id \
         JOIN songs s ON s.id = sh.song_id_fk",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
