//! The migration that made `songs.id` a UUID v7 (`song-public-id`), run down
//! and up. It lives here rather than beside the code it covers because it
//! drives the migrator itself, which no module owns. The sync's id-stability
//! tests sit with `catalog_sync`.

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
    let owners = sheet_owners(&pool).await;
    assert_eq!(owners.len(), 2);

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
    assert_eq!(sheet_owners(&pool).await, owners);

    migrator.run(&pool).await.expect("up again");
    assert_eq!(attached(&pool).await, 2);
    assert_eq!(sheet_owners(&pool).await, owners);
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

/// Each sheet with the song it belongs to, by natural keys, so the mapping
/// can be compared across a change of key type.
async fn sheet_owners(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT sh.sheet_expr, s.song_id FROM sheets sh \
         JOIN songs s ON s.id = sh.song_id_fk ORDER BY sh.sheet_expr",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}
