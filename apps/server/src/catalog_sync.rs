//! Differential catalog sync: applies an upstream payload to the canonical
//! tables by diffing, never by reloading.
//!
//! **No statement here may DELETE from `songs`, `sheets` or `charts`.**
//! `sheets.song_id_fk` and `charts.sheet_id` are both ON DELETE CASCADE, so
//! deleting a song destroys the chart text hanging off it —
//! see docs/work/catalog-sync/spec.md.

use crate::upstream::{RawData, parse_date};
use sqlx::PgPool;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncStats {
    pub songs_inserted: i64,
    pub songs_updated: i64,
    pub sheets_inserted: i64,
    pub sheets_updated: i64,
    pub songs_vanished: i64,
    pub sheets_vanished: i64,
    pub songs_skipped_no_id: i64,
    pub revision: i64,
}

pub async fn apply(pool: &PgPool, data: &RawData) -> Result<SyncStats, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut stats = SyncStats::default();

    // catalog_meta is a singleton guarded by a CHECK constraint, so the row is
    // created on first sync and updated thereafter. revision advances every run;
    // last_full_reload_revision is deliberately left at whatever it already is.
    let previous: Option<i64> = sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?;
    let revision = previous.unwrap_or(0) + 1;
    stats.revision = revision;

    let update_time: DateTime<Utc> = parse_date(&data.update_time)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(Utc::now);

    sqlx::query(
        "INSERT INTO catalog_meta (id, update_time, revision) VALUES (true, $1, $2) \
         ON CONFLICT (id) DO UPDATE SET update_time = EXCLUDED.update_time, \
                                        revision = EXCLUDED.revision",
    )
    .bind(update_time)
    .bind(revision)
    .execute(&mut *tx)
    .await?;

    // Ordered lookup tables carry no rows anything else references, and their
    // `ordinal` is positional, so replacing them wholesale is both safe and
    // simpler than diffing. This is the one place a DELETE is allowed.
    sqlx::query("DELETE FROM categories")
        .execute(&mut *tx)
        .await?;
    for (i, c) in data.categories.iter().enumerate() {
        sqlx::query("INSERT INTO categories (category, ordinal) VALUES ($1, $2)")
            .bind(&c.category)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("DELETE FROM versions")
        .execute(&mut *tx)
        .await?;
    for (i, v) in data.versions.iter().enumerate() {
        sqlx::query(
            "INSERT INTO versions (version, abbr, release_date, ordinal) VALUES ($1, $2, $3, $4)",
        )
        .bind(&v.version)
        .bind(&v.abbr)
        .bind(parse_date(&v.release_date))
        .bind(i as i32)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM types").execute(&mut *tx).await?;
    for (i, t) in data.types.iter().enumerate() {
        sqlx::query(
            "INSERT INTO types (type, name, abbr, icon_url, icon_height, ordinal) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&t.r#type)
        .bind(&t.name)
        .bind(&t.abbr)
        .bind(&t.icon_url)
        .bind(t.icon_height)
        .bind(i as i32)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM difficulties")
        .execute(&mut *tx)
        .await?;
    for (i, d) in data.difficulties.iter().enumerate() {
        sqlx::query(
            "INSERT INTO difficulties (difficulty, name, color, icon_url, icon_height, ordinal) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&d.difficulty)
        .bind(&d.name)
        .bind(&d.color)
        .bind(&d.icon_url)
        .bind(d.icon_height)
        .bind(i as i32)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("DELETE FROM regions").execute(&mut *tx).await?;
    for (i, r) in data.regions.iter().enumerate() {
        sqlx::query("INSERT INTO regions (region, name, ordinal) VALUES ($1, $2, $3)")
            .bind(&r.region)
            .bind(&r.name)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upstream::RawData;
    use sqlx::PgPool;

    fn payload(json: &str) -> RawData {
        serde_json::from_str(json).unwrap()
    }

    #[sqlx::test]
    async fn first_sync_populates_meta_and_lookups(pool: PgPool) -> sqlx::Result<()> {
        let data = payload(
            r#"{
                "updateTime": "2026-09-11",
                "categories": [{ "category": "pops" }, { "category": "niconico" }],
                "regions": [{ "region": "jp", "name": "Japan" }]
            }"#,
        );

        let stats = apply(&pool, &data).await?;
        assert_eq!(stats.revision, 1);

        let categories: Vec<(String, i32)> =
            sqlx::query_as("SELECT category, ordinal FROM categories ORDER BY ordinal")
                .fetch_all(&pool)
                .await?;
        assert_eq!(
            categories,
            vec![("pops".to_string(), 0), ("niconico".to_string(), 1)]
        );

        let (revision, last_full): (i64, i64) =
            sqlx::query_as("SELECT revision, last_full_reload_revision FROM catalog_meta")
                .fetch_one(&pool)
                .await?;
        assert_eq!(revision, 1);
        // Never advanced — this is what keeps /sync/delta working across syncs.
        assert_eq!(last_full, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn second_sync_advances_the_revision_and_replaces_lookups(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let first = payload(r#"{ "categories": [{ "category": "pops" }] }"#);
        apply(&pool, &first).await?;

        let second = payload(r#"{ "categories": [{ "category": "gamemusic" }] }"#);
        let stats = apply(&pool, &second).await?;
        assert_eq!(stats.revision, 2);

        let categories: Vec<(String,)> =
            sqlx::query_as("SELECT category FROM categories ORDER BY ordinal")
                .fetch_all(&pool)
                .await?;
        assert_eq!(categories, vec![("gamemusic".to_string(),)]);

        let (_, last_full): (i64, i64) =
            sqlx::query_as("SELECT revision, last_full_reload_revision FROM catalog_meta")
                .fetch_one(&pool)
                .await?;
        assert_eq!(last_full, 0);

        Ok(())
    }
}
