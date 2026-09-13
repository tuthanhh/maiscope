//! Differential catalog sync: applies an upstream payload to the canonical
//! tables by diffing, never by reloading.
//!
//! **No statement here may DELETE from `songs`, `sheets` or `charts`.**
//! `sheets.song_id_fk` and `charts.sheet_id` are both ON DELETE CASCADE, so
//! deleting a song destroys the chart text hanging off it —
//! see docs/work/catalog-sync/spec.md.

use crate::upstream::{RawData, RawOverride, RawSheet, parse_date, parse_update_time, sheet_expr};
use sqlx::types::Json;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use std::collections::BTreeMap;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncStats {
    pub songs_inserted: i64,
    pub songs_updated: i64,
    pub sheets_inserted: i64,
    pub sheets_updated: i64,
    /// Rows this run logged as vanished for the first time.
    pub songs_vanished: i64,
    pub sheets_vanished: i64,
    /// Every row ever logged as vanished. Without this the log reads
    /// "vanished 0" forever after the first miss, which reads as "nothing is
    /// missing" when the truth is "nothing newly went missing".
    pub songs_vanished_total: i64,
    pub sheets_vanished_total: i64,
    pub songs_skipped_no_id: i64,
    /// The revision rows changed by this run were stamped with. Equal to
    /// `catalog_meta.revision` afterwards only when the run changed something —
    /// a no-op run deliberately leaves the counter where it was.
    pub revision: i64,
    /// Whether `catalog_meta.revision` was advanced to [`Self::revision`].
    pub revision_advanced: bool,
}

impl SyncStats {
    /// Did this run change anything a client could observe? Drives the
    /// conditional `catalog_meta.revision` write-back — see [`apply`].
    fn changed_anything(&self) -> bool {
        self.songs_inserted > 0
            || self.songs_updated > 0
            || self.sheets_inserted > 0
            || self.sheets_updated > 0
            || self.songs_vanished > 0
            || self.sheets_vanished > 0
    }
}

/// The five wholesale-replaced lookup tables, all of which carry an `ordinal`.
const LOOKUP_TABLES: [&str; 5] = ["categories", "versions", "types", "difficulties", "regions"];

pub async fn apply(pool: &PgPool, data: &RawData) -> Result<SyncStats, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut stats = SyncStats::default();

    let update_time: DateTime<Utc> = parse_update_time(&data.update_time).unwrap_or_else(Utc::now);

    // catalog_meta is a singleton guarded by a CHECK constraint, so the row is
    // created on first sync and updated thereafter. `update_time` always
    // reflects upstream; `revision` is *not* touched here — it is advanced at
    // the end of this function, and only if the run actually changed something.
    // last_full_reload_revision is deliberately left at whatever it already is.
    //
    // This upsert doubles as the lock that makes read-now / write-later safe:
    // it takes a row-level exclusive lock on the singleton held until commit,
    // so no concurrent writer (`apply_chart_revision`, another sync) can slip a
    // bump in between the read below and the write-back at the end. Reading
    // with a bare SELECT instead would let the final write regress their value.
    let previous: i64 = sqlx::query_scalar(
        "INSERT INTO catalog_meta (id, update_time, revision) VALUES (true, $1, 0) \
         ON CONFLICT (id) DO UPDATE SET update_time = EXCLUDED.update_time \
         RETURNING revision",
    )
    .bind(update_time)
    .fetch_one(&mut *tx)
    .await?;
    let revision = previous + 1;
    stats.revision = revision;

    // The lookup tables are replaced wholesale, so unlike songs and sheets they
    // have no per-row `IS DISTINCT FROM` guard to report a change. Digest them
    // before and after: without this, a renamed category would be applied but
    // never advance `catalog_meta.revision`, so no client would refetch it.
    let mut lookup_digests_before = Vec::with_capacity(LOOKUP_TABLES.len());
    for table in LOOKUP_TABLES {
        lookup_digests_before.push(lookup_digest(&mut tx, table).await?);
    }

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

    let mut lookups_changed = false;
    for (table, before) in LOOKUP_TABLES.iter().zip(&lookup_digests_before) {
        if &lookup_digest(&mut tx, table).await? != before {
            lookups_changed = true;
        }
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
               source_index = EXCLUDED.source_index, revision = EXCLUDED.revision, \
               updated_at = now() \
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
                   revision = EXCLUDED.revision, updated_at = now() \
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

            // A suppressed DO UPDATE returns no row. The sub-tables still have
            // to be reconciled — `noteCounts`, `regions` and `regionOverrides`
            // change upstream *independently* of the scalar columns in the
            // guard above (note counts get backfilled days after a song ships;
            // a region flips to available), and all three are served by
            // /catalog. Skipping them here froze them at first insert until
            // some unrelated scalar happened to change.
            let scalar_changed = sheet_pk.is_some();
            let stored = read_sub_tables(&mut tx, &expr).await?;

            if sub_tables_differ(&stored, sheet) {
                replace_sub_tables(&mut tx, stored.sheet_id, sheet).await?;

                // A change the client cannot see is only half applied: without
                // this bump /sync/delta would never report the sheet. Only
                // reached on a real difference, so re-writing identical
                // content still bumps nothing.
                if !scalar_changed {
                    sqlx::query(
                        "UPDATE sheets SET revision = $1, updated_at = now() WHERE id = $2",
                    )
                    .bind(revision)
                    .bind(stored.sheet_id)
                    .execute(&mut *tx)
                    .await?;
                    stats.sheets_updated += 1;
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

    stats.songs_vanished_total = sqlx::query_scalar("SELECT COUNT(*) FROM deleted_songs")
        .fetch_one(&mut *tx)
        .await?;
    stats.sheets_vanished_total = sqlx::query_scalar("SELECT COUNT(*) FROM deleted_sheets")
        .fetch_one(&mut *tx)
        .await?;

    // The whole point of this feature is that a run which changed nothing costs
    // clients nothing. `catalog_meta.revision` feeds the /catalog ETag
    // (sha256(revision:updateTime)), so advancing it on a no-op run would make
    // every client refetch 4.7MB daily — the exact cost being removed here.
    // Rows were still *stamped* with `revision`; only this write-back is
    // conditional.
    //
    // GREATEST is belt-and-braces: the singleton is locked for this whole
    // transaction (see the upsert at the top), so `revision` cannot have moved
    // under us — but a future refactor that drops the lock would otherwise turn
    // this into a silent regression of another writer's bump.
    stats.revision_advanced = stats.changed_anything() || lookups_changed;
    if stats.revision_advanced {
        sqlx::query("UPDATE catalog_meta SET revision = GREATEST(revision, $1)")
            .bind(revision)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(stats)
}

/// md5 over a lookup table's whole contents, in `ordinal` order. Used only to
/// compare the table against itself before and after a wholesale replace, so
/// the exact row-text format does not matter — only that it is stable within
/// one transaction.
async fn lookup_digest(conn: &mut PgConnection, table: &str) -> Result<String, sqlx::Error> {
    // A table name cannot be a bind parameter. `table` is one of the five
    // literals in LOOKUP_TABLES and never reaches this function from a request
    // or from the payload, so the interpolation is audited-safe — which is
    // exactly what AssertSqlSafe asserts.
    let sql = format!(
        "SELECT COALESCE(md5(string_agg(t::text, '|' ORDER BY t.ordinal)), '') FROM {table} t"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(conn)
        .await
}

/// A sheet's three sub-tables as stored, shaped to compare directly against the
/// upstream payload.
struct StoredSubTables {
    sheet_id: i64,
    note_counts: BTreeMap<String, Option<i64>>,
    regions: BTreeMap<String, bool>,
    region_overrides: BTreeMap<String, RawOverride>,
}

/// Reads the sheet's id and all three sub-tables in **one** round trip, as
/// jsonb aggregates decoded into the same types the payload parses into.
///
/// Comparing typed maps in Rust rather than digesting both sides in SQL is
/// deliberate: a digest would have to reproduce Postgres's float and NULL text
/// formatting byte-for-byte to agree, and any drift there reads as "changed"
/// forever. jsonb round-trips `double precision` through its shortest
/// round-trip representation, so `==` on the decoded `f64` is exact.
async fn read_sub_tables(
    conn: &mut PgConnection,
    sheet_expr: &str,
) -> Result<StoredSubTables, sqlx::Error> {
    type Row = (
        i64,
        Json<BTreeMap<String, Option<i64>>>,
        Json<BTreeMap<String, bool>>,
        Json<BTreeMap<String, RawOverride>>,
    );

    let (sheet_id, note_counts, regions, region_overrides): Row = sqlx::query_as(
        "SELECT s.id, \
           (SELECT COALESCE(jsonb_object_agg(n.key, n.value), '{}'::jsonb) \
              FROM sheet_note_counts n WHERE n.sheet_id = s.id), \
           (SELECT COALESCE(jsonb_object_agg(r.region, r.available), '{}'::jsonb) \
              FROM sheet_regions r WHERE r.sheet_id = s.id), \
           (SELECT COALESCE(jsonb_object_agg(o.region, jsonb_build_object( \
                     'level', o.level, 'levelValue', o.level_value, \
                     'internalLevel', o.internal_level, \
                     'internalLevelValue', o.internal_level_value, \
                     'noteDesigner', o.note_designer)), '{}'::jsonb) \
              FROM sheet_region_overrides o WHERE o.sheet_id = s.id) \
         FROM sheets s WHERE s.sheet_expr = $1",
    )
    .bind(sheet_expr)
    .fetch_one(conn)
    .await?;

    Ok(StoredSubTables {
        sheet_id,
        note_counts: note_counts.0,
        regions: regions.0,
        region_overrides: region_overrides.0,
    })
}

/// An absent map upstream and an empty one are indistinguishable once stored —
/// both are zero rows — so they must compare equal, or every sheet without
/// `regionOverrides` would look changed on every run.
fn maps_differ<V: PartialEq>(
    stored: &BTreeMap<String, V>,
    incoming: &Option<BTreeMap<String, V>>,
) -> bool {
    match incoming {
        Some(incoming) => stored != incoming,
        None => !stored.is_empty(),
    }
}

fn sub_tables_differ(stored: &StoredSubTables, sheet: &RawSheet) -> bool {
    maps_differ(&stored.note_counts, &sheet.note_counts)
        || maps_differ(&stored.regions, &sheet.regions)
        || maps_differ(&stored.region_overrides, &sheet.region_overrides)
}

/// Sub-tables key off `sheet_id` with no natural key of their own, so they are
/// replaced per sheet.
///
/// **Every DELETE here is scoped to one `sheet_id` and must stay that way.** A
/// bare `DELETE FROM` on any of these three would be a different statement with
/// a very different blast radius. Scoped like this the deletes cascade to
/// nothing: `charts` hangs off `sheets`, which this module never deletes.
async fn replace_sub_tables(
    conn: &mut PgConnection,
    sheet_id: i64,
    sheet: &RawSheet,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sheet_note_counts WHERE sheet_id = $1")
        .bind(sheet_id)
        .execute(&mut *conn)
        .await?;
    if let Some(counts) = &sheet.note_counts {
        for (key, value) in counts {
            sqlx::query("INSERT INTO sheet_note_counts (sheet_id, key, value) VALUES ($1, $2, $3)")
                .bind(sheet_id)
                .bind(key)
                .bind(value)
                .execute(&mut *conn)
                .await?;
        }
    }

    sqlx::query("DELETE FROM sheet_regions WHERE sheet_id = $1")
        .bind(sheet_id)
        .execute(&mut *conn)
        .await?;
    if let Some(regions) = &sheet.regions {
        for (region, available) in regions {
            sqlx::query(
                "INSERT INTO sheet_regions (sheet_id, region, available) VALUES ($1, $2, $3)",
            )
            .bind(sheet_id)
            .bind(region)
            .bind(available)
            .execute(&mut *conn)
            .await?;
        }
    }

    sqlx::query("DELETE FROM sheet_region_overrides WHERE sheet_id = $1")
        .bind(sheet_id)
        .execute(&mut *conn)
        .await?;
    if let Some(overrides) = &sheet.region_overrides {
        for (region, ov) in overrides {
            sqlx::query(
                "INSERT INTO sheet_region_overrides \
                 (sheet_id, region, level, level_value, internal_level, \
                  internal_level_value, note_designer) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(sheet_id)
            .bind(region)
            .bind(&ov.level)
            .bind(ov.level_value)
            .bind(&ov.internal_level)
            .bind(ov.internal_level_value)
            .bind(&ov.note_designer)
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}

/// Refuse implausible payloads before any write. The failure this guards is a
/// truncated or malformed upstream response landing in production unattended —
/// see docs/work/catalog-sync/spec.md.
pub fn sanity_check(incoming: usize, current: i64) -> Result<(), String> {
    if incoming == 0 {
        return Err("upstream returned zero songs".to_string());
    }
    if current > 0 {
        // `current` is `COUNT(*) FROM songs`, and this sync never deletes, so
        // that count only ever grows: the floor ratchets. A single 15% upstream
        // loss is caught, but a loss spread over enough runs to stay inside 10%
        // each time is absorbed silently, and the floor then sits permanently
        // above the true upstream size. Deliberate for now — tightening it needs
        // a notion of "expected current size" the schema does not carry.
        let floor = (current as f64 * 0.9).floor() as usize;
        if incoming <= floor {
            return Err(format!(
                "upstream returned {incoming} songs, below the {floor} floor (90% of {current})"
            ));
        }
    }
    Ok(())
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

    // The live feed's real updateTime shape. Before this, parse_date returned
    // None for it and the fallback stamped Utc::now() on every single run, so
    // the ETag changed daily and no test noticed.
    #[sqlx::test]
    async fn the_live_update_time_is_stored_not_replaced_with_now(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let data = payload(r#"{ "updateTime": "2026-09-11T01:24:07.945Z" }"#);
        apply(&pool, &data).await?;

        let stored: String = sqlx::query_scalar(
            "SELECT to_char(update_time AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS') \
             FROM catalog_meta",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(stored, "2026-09-11T01:24:07");
        Ok(())
    }

    async fn catalog_meta_revision(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap()
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
        // A lookup table changed, so the counter must move even though no song
        // or sheet did — the lookups are what /catalog's filter UI is built from.
        assert!(stats.revision_advanced);
        assert_eq!(catalog_meta_revision(&pool).await, 2);

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

        // Still revision 1 — and so is catalog_meta, see the test below.
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

    // The /catalog ETag is sha256(revision:updateTime). Advancing `revision` on
    // a run that changed nothing makes every client refetch 4.7MB a day, which
    // is the exact cost this feature exists to remove.
    #[sqlx::test]
    async fn an_identical_second_sync_leaves_catalog_meta_revision_alone(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        let after_first = catalog_meta_revision(&pool).await;
        assert_eq!(after_first, 1);

        let stats = apply(&pool, &payload(ONE_SONG)).await?;
        assert!(!stats.revision_advanced);
        assert_eq!(
            catalog_meta_revision(&pool).await,
            after_first,
            "a no-op run must not churn the ETag"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn a_changed_row_does_advance_catalog_meta_revision(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        let retitled = ONE_SONG.replace("Example Song", "Renamed Song");
        let stats = apply(&pool, &payload(&retitled)).await?;

        assert!(stats.revision_advanced);
        assert_eq!(catalog_meta_revision(&pool).await, 2);
        Ok(())
    }

    // A concurrent writer (apply_chart_revision, or a second sync) must not have
    // its bump overwritten by this one's read-then-write. The singleton row is
    // locked for the whole transaction, so the interleaving is serialised rather
    // than lost: the later run reads the committed value and builds on it.
    #[sqlx::test]
    async fn a_concurrent_bump_is_not_regressed(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        assert_eq!(catalog_meta_revision(&pool).await, 1);

        // Stand in for any other writer on the shared counter.
        sqlx::query("UPDATE catalog_meta SET revision = revision + 1")
            .execute(&pool)
            .await?;
        assert_eq!(catalog_meta_revision(&pool).await, 2);

        let retitled = ONE_SONG.replace("Example Song", "Renamed Song");
        apply(&pool, &payload(&retitled)).await?;

        assert_eq!(
            catalog_meta_revision(&pool).await,
            3,
            "the sync must build on the other writer's value, not overwrite it"
        );
        let song_revision: i64 =
            sqlx::query_scalar("SELECT revision FROM songs WHERE song_id = 'Example'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(song_revision, 3, "the row must carry a fetchable revision");
        Ok(())
    }

    const SONG_WITH_SUB_TABLES: &str = r#"{
        "songs": [{
            "songId": "Example",
            "title": "Example Song",
            "sheets": [{
                "type": "dx", "difficulty": "master", "level": "14+",
                "noteCounts": { "tap": 100, "total": 400, "touch": null },
                "regions": { "jp": true, "intl": false },
                "regionOverrides": { "intl": { "level": "14", "levelValue": 14.0 } }
            }]
        }]
    }"#;

    async fn note_counts(pool: &PgPool) -> Vec<(String, Option<i32>)> {
        sqlx::query_as("SELECT key, value FROM sheet_note_counts ORDER BY key")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    async fn sheet_revision(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT revision FROM sheets")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // The regression this guards: noteCounts / regions / regionOverrides change
    // upstream *independently* of the scalar columns in the sheets upsert's
    // IS DISTINCT FROM guard. Skipping the sub-tables whenever that guard
    // suppressed the update froze them at first insert — note counts backfilled
    // days after a song ships would never land, and /catalog serves all three.
    #[sqlx::test]
    async fn a_sub_table_only_change_is_applied_and_bumps_the_sheet(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;
        assert_eq!(sheet_revision(&pool).await, 1);

        // Only noteCounts differs; every scalar column is byte-identical.
        let backfilled = SONG_WITH_SUB_TABLES.replace(r#""tap": 100"#, r#""tap": 123"#);
        let stats = apply(&pool, &payload(&backfilled)).await?;

        assert_eq!(
            stats.sheets_updated, 1,
            "a sub-table-only change is still a change to the sheet"
        );
        assert_eq!(
            note_counts(&pool).await,
            vec![
                ("tap".to_string(), Some(123)),
                ("total".to_string(), Some(400)),
                ("touch".to_string(), None),
            ]
        );
        // Applied but not visible to /sync/delta would be a half-fix.
        assert_eq!(sheet_revision(&pool).await, 2);
        assert_eq!(catalog_meta_revision(&pool).await, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn a_regions_only_change_is_applied_and_bumps_the_sheet(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;

        let released = SONG_WITH_SUB_TABLES.replace(r#""intl": false"#, r#""intl": true"#);
        let stats = apply(&pool, &payload(&released)).await?;

        assert_eq!(stats.sheets_updated, 1);
        let available: bool =
            sqlx::query_scalar("SELECT available FROM sheet_regions WHERE region = 'intl'")
                .fetch_one(&pool)
                .await?;
        assert!(available, "a region flipping to available must land");
        assert_eq!(sheet_revision(&pool).await, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn a_region_override_only_change_is_applied_and_bumps_the_sheet(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;

        let rerated =
            SONG_WITH_SUB_TABLES.replace(r#""levelValue": 14.0"#, r#""levelValue": 14.3"#);
        let stats = apply(&pool, &payload(&rerated)).await?;

        assert_eq!(stats.sheets_updated, 1);
        let level_value: Option<f64> = sqlx::query_scalar(
            "SELECT level_value FROM sheet_region_overrides WHERE region = 'intl'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(level_value, Some(14.3));
        assert_eq!(sheet_revision(&pool).await, 2);
        Ok(())
    }

    // The other half of the requirement: re-writing identical sub-table content
    // must bump nothing. A float, a NULL note count and an absent-vs-empty map
    // are the three places a naive comparison reports a spurious difference.
    #[sqlx::test]
    async fn an_identical_sub_table_rerun_bumps_nothing(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;
        let stats = apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;

        assert_eq!(stats.sheets_updated, 0);
        assert_eq!(stats.songs_updated, 0);
        assert!(!stats.revision_advanced);
        assert_eq!(sheet_revision(&pool).await, 1);
        assert_eq!(catalog_meta_revision(&pool).await, 1);
        Ok(())
    }

    // ONE_SONG carries no sub-table maps at all. Absent upstream and empty in
    // the database are indistinguishable once stored, so they must compare equal
    // — otherwise every sheet without regionOverrides looks changed every run.
    #[sqlx::test]
    async fn absent_sub_tables_do_not_look_like_a_change(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        let stats = apply(&pool, &payload(ONE_SONG)).await?;

        assert_eq!(stats.sheets_updated, 0);
        assert!(!stats.revision_advanced);
        Ok(())
    }

    // Removing a note count upstream must remove the row, not leave it behind.
    #[sqlx::test]
    async fn a_removed_sub_table_entry_is_dropped(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;

        let trimmed = SONG_WITH_SUB_TABLES.replace(r#", "touch": null"#, "");
        apply(&pool, &payload(&trimmed)).await?;

        assert_eq!(
            note_counts(&pool).await,
            vec![
                ("tap".to_string(), Some(100)),
                ("total".to_string(), Some(400)),
            ]
        );
        Ok(())
    }

    // A genuinely changed row must not keep a stale updated_at.
    #[sqlx::test]
    async fn a_changed_row_refreshes_updated_at(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;
        sqlx::query("UPDATE songs SET updated_at = now() - interval '10 days'")
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE sheets SET updated_at = now() - interval '10 days'")
            .execute(&pool)
            .await?;

        let retitled = ONE_SONG
            .replace("Example Song", "Renamed Song")
            .replace(r#""level": "14+""#, r#""level": "15""#);
        apply(&pool, &payload(&retitled)).await?;

        let song_is_fresh: bool =
            sqlx::query_scalar("SELECT updated_at > now() - interval '1 minute' FROM songs")
                .fetch_one(&pool)
                .await?;
        let sheet_is_fresh: bool =
            sqlx::query_scalar("SELECT updated_at > now() - interval '1 minute' FROM sheets")
                .fetch_one(&pool)
                .await?;
        assert!(song_is_fresh, "songs.updated_at must be refreshed");
        assert!(sheet_is_fresh, "sheets.updated_at must be refreshed");
        Ok(())
    }

    // "vanished 0" reads as "nothing is missing"; the truth after the first miss
    // is "nothing newly went missing". Both numbers have to be reportable.
    #[sqlx::test]
    async fn vanished_counts_report_new_and_cumulative(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(ONE_SONG)).await?;

        let first_miss = apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;
        assert_eq!(first_miss.songs_vanished, 1);
        assert_eq!(first_miss.songs_vanished_total, 1);

        let second_miss = apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;
        assert_eq!(second_miss.songs_vanished, 0, "nothing newly vanished");
        assert_eq!(
            second_miss.songs_vanished_total, 1,
            "but one row is still missing upstream"
        );
        assert_eq!(second_miss.sheets_vanished_total, 1);
        Ok(())
    }

    // The inviolable constraint: no code path may DELETE from or cascade to
    // songs, sheets or charts. Sub-table reconciliation now runs for every sheet
    // in the payload, including unchanged ones, so this is the path that guards
    // its deletes staying scoped to a single sheet_id.
    #[sqlx::test]
    async fn reconciling_sub_tables_never_touches_chart_text(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(SONG_WITH_SUB_TABLES)).await?;

        let sheet_id: i64 = sqlx::query_scalar("SELECT id FROM sheets")
            .fetch_one(&pool)
            .await?;
        sqlx::query("INSERT INTO charts (sheet_id, sheet_expr, content) VALUES ($1, $2, $3)")
            .bind(sheet_id)
            .bind("Example|dx|master")
            .bind("&title=Example")
            .execute(&pool)
            .await?;

        // A sub-table-only change, then an identical re-run.
        let backfilled = SONG_WITH_SUB_TABLES.replace(r#""tap": 100"#, r#""tap": 123"#);
        apply(&pool, &payload(&backfilled)).await?;
        apply(&pool, &payload(&backfilled)).await?;

        let charts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM charts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            charts, 1,
            "chart text must survive sub-table reconciliation"
        );
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

    #[test]
    fn sanity_check_rejects_an_empty_payload() {
        assert!(sanity_check(0, 1845).is_err());
        assert!(sanity_check(0, 0).is_err(), "empty is never plausible");
    }

    #[test]
    fn sanity_check_allows_a_first_sync_into_an_empty_database() {
        assert!(sanity_check(1845, 0).is_ok());
    }

    #[test]
    fn sanity_check_rejects_a_large_drop() {
        // 90% of 1845 is 1660.5, so 1660 is below the floor and 1661 is above it.
        assert!(sanity_check(1660, 1845).is_err());
        assert!(sanity_check(1661, 1845).is_ok());
    }

    #[test]
    fn sanity_check_allows_growth() {
        assert!(sanity_check(2000, 1845).is_ok());
    }
}
