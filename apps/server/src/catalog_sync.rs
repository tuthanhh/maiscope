//! Differential catalog sync: applies an upstream payload to the canonical
//! tables by diffing, never by reloading.
//!
//! **No statement here may DELETE from `songs`, `sheets` or `charts`.**
//! `sheets.song_id_fk` and `charts.sheet_id` are both ON DELETE CASCADE, so
//! deleting a song destroys the chart text hanging off it —
//! see docs/work/catalog-sync/spec.md.

use crate::upstream::{RawData, parse_date, sheet_expr};
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

    // Songs and sheets are matched on their natural keys — songs.song_id and
    // sheets.sheet_expr, both UNIQUE. The `WHERE ... IS DISTINCT FROM` guard on
    // DO UPDATE is what keeps `revision` still for untouched rows: without it
    // every sync would mark every row changed and /sync/delta would return the
    // entire catalog every time.
    for (si, song) in data.songs.iter().enumerate() {
        // A NULL song_id cannot be matched on a later run (UNIQUE permits many
        // NULLs), so such a row would be re-inserted forever. None exist upstream
        // today; skip and count so it shows up in the log if that changes.
        let Some(song_id) = song.song_id.as_deref() else {
            stats.songs_skipped_no_id += 1;
            continue;
        };

        let song_pk: Option<i64> = sqlx::query_as::<_, (i64, bool)>(
            "INSERT INTO songs \
             (song_id, song_no, category, title, artist, bpm, image_name, version, \
              release_date, is_new, is_locked, comment, source_index, revision) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             ON CONFLICT (song_id) DO UPDATE SET \
               song_no = EXCLUDED.song_no, category = EXCLUDED.category, \
               title = EXCLUDED.title, artist = EXCLUDED.artist, bpm = EXCLUDED.bpm, \
               image_name = EXCLUDED.image_name, version = EXCLUDED.version, \
               release_date = EXCLUDED.release_date, is_new = EXCLUDED.is_new, \
               is_locked = EXCLUDED.is_locked, comment = EXCLUDED.comment, \
               source_index = EXCLUDED.source_index, revision = EXCLUDED.revision \
             WHERE ROW(songs.song_no, songs.category, songs.title, songs.artist, songs.bpm, \
                       songs.image_name, songs.version, songs.release_date, songs.is_new, \
                       songs.is_locked, songs.comment, songs.source_index) \
                   IS DISTINCT FROM \
                   ROW(EXCLUDED.song_no, EXCLUDED.category, EXCLUDED.title, EXCLUDED.artist, \
                       EXCLUDED.bpm, EXCLUDED.image_name, EXCLUDED.version, \
                       EXCLUDED.release_date, EXCLUDED.is_new, EXCLUDED.is_locked, \
                       EXCLUDED.comment, EXCLUDED.source_index) \
             RETURNING id, (xmax = 0) AS inserted",
        )
        .bind(song_id)
        .bind(si as i32 + 1)
        .bind(&song.category)
        .bind(&song.title)
        .bind(&song.artist)
        .bind(song.bpm)
        .bind(&song.image_name)
        .bind(&song.version)
        .bind(parse_date(&song.release_date))
        .bind(song.is_new)
        .bind(song.is_locked)
        .bind(&song.comment)
        .bind(si as i32)
        .bind(revision)
        .fetch_optional(&mut *tx)
        .await?
        .map(|(id, inserted)| {
            if inserted {
                stats.songs_inserted += 1;
            } else {
                stats.songs_updated += 1;
            }
            id
        });

        // A suppressed DO UPDATE returns no row, so re-read the id to reach the
        // sheets. This is the unchanged-song path and stays a single indexed
        // lookup on a UNIQUE column.
        let song_pk: i64 = match song_pk {
            Some(id) => id,
            None => {
                sqlx::query_scalar("SELECT id FROM songs WHERE song_id = $1")
                    .bind(song_id)
                    .fetch_one(&mut *tx)
                    .await?
            }
        };

        for (shi, sheet) in song.sheets.iter().enumerate() {
            let expr = sheet_expr(&song.song_id, &sheet.r#type, &sheet.difficulty);

            let sheet_pk: Option<i64> = sqlx::query_as::<_, (i64, bool)>(
                "INSERT INTO sheets \
                 (song_id_fk, sheet_expr, type, difficulty, level, level_value, \
                  internal_level, internal_level_value, note_designer, is_special, \
                  source_index, revision) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
                 ON CONFLICT (sheet_expr) DO UPDATE SET \
                   song_id_fk = EXCLUDED.song_id_fk, type = EXCLUDED.type, \
                   difficulty = EXCLUDED.difficulty, level = EXCLUDED.level, \
                   level_value = EXCLUDED.level_value, \
                   internal_level = EXCLUDED.internal_level, \
                   internal_level_value = EXCLUDED.internal_level_value, \
                   note_designer = EXCLUDED.note_designer, \
                   is_special = EXCLUDED.is_special, source_index = EXCLUDED.source_index, \
                   revision = EXCLUDED.revision \
                 WHERE ROW(sheets.song_id_fk, sheets.type, sheets.difficulty, sheets.level, \
                           sheets.level_value, sheets.internal_level, \
                           sheets.internal_level_value, sheets.note_designer, \
                           sheets.is_special, sheets.source_index) \
                       IS DISTINCT FROM \
                       ROW(EXCLUDED.song_id_fk, EXCLUDED.type, EXCLUDED.difficulty, \
                           EXCLUDED.level, EXCLUDED.level_value, EXCLUDED.internal_level, \
                           EXCLUDED.internal_level_value, EXCLUDED.note_designer, \
                           EXCLUDED.is_special, EXCLUDED.source_index) \
                 RETURNING id, (xmax = 0) AS inserted",
            )
            .bind(song_pk)
            .bind(&expr)
            .bind(&sheet.r#type)
            .bind(&sheet.difficulty)
            .bind(&sheet.level)
            .bind(sheet.level_value)
            .bind(&sheet.internal_level)
            .bind(sheet.internal_level_value)
            .bind(&sheet.note_designer)
            .bind(sheet.is_special)
            .bind(shi as i32)
            .bind(revision)
            .fetch_optional(&mut *tx)
            .await?
            .map(|(id, inserted)| {
                if inserted {
                    stats.sheets_inserted += 1;
                } else {
                    stats.sheets_updated += 1;
                }
                id
            });

            let Some(sheet_pk) = sheet_pk else {
                // Unchanged sheet: its sub-tables are unchanged too.
                continue;
            };

            // Sub-tables key off sheet_id with no natural key of their own, so
            // they are replaced per sheet. Deleting from these cascades to
            // nothing — `charts` hangs off `sheets`, which is never deleted.
            sqlx::query("DELETE FROM sheet_note_counts WHERE sheet_id = $1")
                .bind(sheet_pk)
                .execute(&mut *tx)
                .await?;
            if let Some(counts) = &sheet.note_counts {
                for (key, value) in counts {
                    sqlx::query(
                        "INSERT INTO sheet_note_counts (sheet_id, key, value) VALUES ($1, $2, $3)",
                    )
                    .bind(sheet_pk)
                    .bind(key)
                    .bind(value)
                    .execute(&mut *tx)
                    .await?;
                }
            }

            sqlx::query("DELETE FROM sheet_regions WHERE sheet_id = $1")
                .bind(sheet_pk)
                .execute(&mut *tx)
                .await?;
            if let Some(regions) = &sheet.regions {
                for (region, available) in regions {
                    sqlx::query(
                        "INSERT INTO sheet_regions (sheet_id, region, available) \
                         VALUES ($1, $2, $3)",
                    )
                    .bind(sheet_pk)
                    .bind(region)
                    .bind(available)
                    .execute(&mut *tx)
                    .await?;
                }
            }

            sqlx::query("DELETE FROM sheet_region_overrides WHERE sheet_id = $1")
                .bind(sheet_pk)
                .execute(&mut *tx)
                .await?;
            if let Some(overrides) = &sheet.region_overrides {
                for (region, ov) in overrides {
                    sqlx::query(
                        "INSERT INTO sheet_region_overrides \
                         (sheet_id, region, level, level_value, internal_level, \
                          internal_level_value, note_designer) \
                         VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    )
                    .bind(sheet_pk)
                    .bind(region)
                    .bind(&ov.level)
                    .bind(ov.level_value)
                    .bind(&ov.internal_level)
                    .bind(ov.internal_level_value)
                    .bind(&ov.note_designer)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }

    // Rows the database holds that upstream no longer lists. They are logged for
    // /sync/delta and left in place — deleting a sheet would cascade to its
    // chart text. Rows already logged are not logged again: the deletion tables
    // are keyed (song_id, revision), so ON CONFLICT DO NOTHING would still write
    // a second entry under a new revision.
    let present_song_ids: Vec<String> = data
        .songs
        .iter()
        .filter_map(|s| s.song_id.clone())
        .collect();
    let present_sheet_exprs: Vec<String> = data
        .songs
        .iter()
        .flat_map(|s| {
            s.sheets
                .iter()
                .map(move |sh| sheet_expr(&s.song_id, &sh.r#type, &sh.difficulty))
        })
        .collect();

    stats.songs_vanished = sqlx::query_scalar::<_, i32>(
        "INSERT INTO deleted_songs (song_id, revision) \
         SELECT s.song_id, $2 FROM songs s \
         WHERE s.song_id IS NOT NULL \
           AND s.song_id <> ALL($1) \
           AND NOT EXISTS (SELECT 1 FROM deleted_songs d WHERE d.song_id = s.song_id) \
         RETURNING 1",
    )
    .bind(&present_song_ids)
    .bind(revision)
    .fetch_all(&mut *tx)
    .await?
    .len() as i64;

    stats.sheets_vanished = sqlx::query_scalar::<_, i32>(
        "INSERT INTO deleted_sheets (sheet_expr, revision) \
         SELECT s.sheet_expr, $2 FROM sheets s \
         WHERE s.sheet_expr <> ALL($1) \
           AND NOT EXISTS (SELECT 1 FROM deleted_sheets d WHERE d.sheet_expr = s.sheet_expr) \
         RETURNING 1",
    )
    .bind(&present_sheet_exprs)
    .bind(revision)
    .fetch_all(&mut *tx)
    .await?
    .len() as i64;

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

    const ONE_SONG: &str = r#"{
        "songs": [{
            "songId": "Example",
            "title": "Example Song",
            "artist": "Someone",
            "bpm": 180.0,
            "sheets": [{
                "type": "dx", "difficulty": "master",
                "level": "14+", "levelValue": 14.7
            }]
        }]
    }"#;

    #[sqlx::test]
    async fn inserts_songs_and_sheets_on_first_sync(pool: PgPool) -> sqlx::Result<()> {
        let stats = apply(&pool, &payload(ONE_SONG)).await?;

        assert_eq!(stats.songs_inserted, 1);
        assert_eq!(stats.sheets_inserted, 1);
        assert_eq!(stats.songs_updated, 0);

        let (title, revision): (String, i64) =
            sqlx::query_as("SELECT title, revision FROM songs WHERE song_id = 'Example'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(title, "Example Song");
        assert_eq!(revision, 1);

        let (expr, sheet_revision): (String, i64) =
            sqlx::query_as("SELECT sheet_expr, revision FROM sheets")
                .fetch_one(&pool)
                .await?;
        assert_eq!(expr, "Example|dx|master");
        assert_eq!(sheet_revision, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn identical_second_sync_bumps_no_row_revisions(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        let stats = apply(&pool, &payload(ONE_SONG)).await?;

        assert_eq!(stats.songs_inserted, 0);
        assert_eq!(stats.songs_updated, 0, "unchanged song must not be updated");
        assert_eq!(
            stats.sheets_updated, 0,
            "unchanged sheet must not be updated"
        );

        // Still revision 1 even though catalog_meta is now at 2.
        let song_revision: i64 =
            sqlx::query_scalar("SELECT revision FROM songs WHERE song_id = 'Example'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(song_revision, 1);

        let sheet_revision: i64 = sqlx::query_scalar("SELECT revision FROM sheets")
            .fetch_one(&pool)
            .await?;
        assert_eq!(sheet_revision, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn a_changed_field_bumps_only_that_row(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;

        let retitled = ONE_SONG.replace("Example Song", "Renamed Song");
        let stats = apply(&pool, &payload(&retitled)).await?;

        assert_eq!(stats.songs_updated, 1);
        assert_eq!(stats.sheets_updated, 0, "the sheet did not change");

        let (title, revision): (String, i64) =
            sqlx::query_as("SELECT title, revision FROM songs WHERE song_id = 'Example'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(title, "Renamed Song");
        assert_eq!(revision, 2);

        Ok(())
    }

    #[sqlx::test]
    async fn songs_without_an_id_are_skipped_and_counted(pool: PgPool) -> sqlx::Result<()> {
        let data = payload(r#"{ "songs": [{ "title": "No Id", "sheets": [] }] }"#);
        let stats = apply(&pool, &data).await?;

        assert_eq!(stats.songs_skipped_no_id, 1);
        assert_eq!(stats.songs_inserted, 0);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
            .fetch_one(&pool)
            .await?;
        assert_eq!(count, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn sync_never_touches_chart_text(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;

        let sheet_id: i64 = sqlx::query_scalar("SELECT id FROM sheets")
            .fetch_one(&pool)
            .await?;
        sqlx::query("INSERT INTO charts (sheet_id, sheet_expr, content) VALUES ($1, $2, $3)")
            .bind(sheet_id)
            .bind("Example|dx|master")
            .bind("&title=Example")
            .execute(&pool)
            .await?;

        // A sync where the song vanished entirely from upstream.
        apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;

        let charts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM charts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            charts, 1,
            "chart text must survive a song vanishing upstream"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn a_vanished_song_is_logged_but_not_deleted(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        let stats = apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;

        assert_eq!(stats.songs_vanished, 1);
        assert_eq!(stats.sheets_vanished, 1);

        // The rows are still there — never deleted.
        let songs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
            .fetch_one(&pool)
            .await?;
        assert_eq!(songs, 1);

        let logged: Vec<(String, i64)> =
            sqlx::query_as("SELECT song_id, revision FROM deleted_songs")
                .fetch_all(&pool)
                .await?;
        assert_eq!(logged, vec![("Example".to_string(), 2)]);

        let logged_sheets: Vec<(String, i64)> =
            sqlx::query_as("SELECT sheet_expr, revision FROM deleted_sheets")
                .fetch_all(&pool)
                .await?;
        assert_eq!(logged_sheets, vec![("Example|dx|master".to_string(), 2)]);

        Ok(())
    }

    #[sqlx::test]
    async fn a_vanished_row_is_logged_once_not_every_sync(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;
        let stats = apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;

        assert_eq!(
            stats.songs_vanished, 0,
            "already logged on the previous sync"
        );

        let entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deleted_songs")
            .fetch_one(&pool)
            .await?;
        assert_eq!(entries, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn a_returning_song_is_updated_normally(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;
        let stats = apply(&pool, &payload(ONE_SONG)).await?;

        // The row never left, so this is neither an insert nor a vanish.
        assert_eq!(stats.songs_inserted, 0);
        assert_eq!(stats.songs_vanished, 0);

        Ok(())
    }
}
