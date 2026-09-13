//! Differential catalog sync: applies an upstream payload to the canonical
//! tables by diffing, never by reloading.
//!
//! **No statement here may DELETE from `songs`, `sheets` or `charts`.**
//! `sheets.song_id_fk` and `charts.sheet_id` are both ON DELETE CASCADE, so
//! deleting a song destroys the chart text hanging off it —
//! see docs/work/catalog-sync/spec.md.
//!
//! **Every statement is set-based: no loop in this module may await inside
//! itself.** The payload is 1.8k songs and 7.3k sheets carrying ~81k sub-table
//! entries, and one round trip per row put a first sync at ~119k sequential
//! round trips — 15 minutes from a GitHub runner to Neon, past
//! `bin/sync_catalog`'s whole-run budget, with the `catalog_meta` row lock held
//! the entire time. Column vectors bound as arrays and zipped back by `unnest`
//! bring the same work to ~15 round trips. A per-row `.execute()` added back
//! into a loop here silently restores the 15 minutes.

use crate::upstream::{
    RawData, RawOverride, RawSheet, RawSong, parse_date, parse_update_time, sheet_expr,
};
use sqlx::Row as _;
use sqlx::types::Json;
use sqlx::types::chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool};
use std::collections::{BTreeMap, HashMap};

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
    let digests_before = lookup_digests(&mut tx).await?;

    replace_lookups(&mut tx, data).await?;

    let lookups_changed = lookup_digests(&mut tx).await? != digests_before;

    // Songs and sheets are matched on their natural keys — songs.song_id and
    // sheets.sheet_expr, both UNIQUE. The `WHERE ... IS DISTINCT FROM` guard on
    // DO UPDATE is what keeps `revision` still for untouched rows: without it
    // every sync would mark every row changed and /sync/delta would return the
    // entire catalog every time.
    let songs = dedupe_songs(data, &mut stats);
    let changed_songs = upsert_songs(&mut tx, &songs, revision).await?;
    for inserted in changed_songs.values() {
        if *inserted {
            stats.songs_inserted += 1;
        } else {
            stats.songs_updated += 1;
        }
    }

    // The upsert's suppressed DO UPDATE returns no row, so the ids of unchanged
    // songs are not in `changed_songs`. Reading the whole map back costs one
    // round trip and covers changed and unchanged rows alike — the per-row
    // fallback lookup it replaces was 1.8k of them.
    let song_ids: Vec<String> = songs
        .iter()
        .map(|(_, s)| s.song_id.clone().unwrap())
        .collect();
    let song_pks: HashMap<String, i64> =
        sqlx::query_as("SELECT song_id, id FROM songs WHERE song_id = ANY($1)")
            .bind(&song_ids)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();

    let sheets = dedupe_sheets(&songs, &song_pks);
    let changed_sheets = upsert_sheets(&mut tx, &sheets, revision).await?;
    for inserted in changed_sheets.values() {
        if *inserted {
            stats.sheets_inserted += 1;
        } else {
            stats.sheets_updated += 1;
        }
    }

    // The sub-tables still have to be reconciled for every sheet in the
    // payload, including ones the scalar guard above left alone: `noteCounts`,
    // `regions` and `regionOverrides` change upstream *independently* of the
    // scalar columns (note counts get backfilled days after a song ships; a
    // region flips to available), and all three are served by /catalog.
    // Skipping them when the guard suppressed the update froze them at first
    // insert until some unrelated scalar happened to change.
    let sheet_exprs: Vec<String> = sheets.iter().map(|s| s.expr.clone()).collect();
    let stored = read_sub_tables(&mut tx, &sheet_exprs).await?;

    let mut dirty: Vec<i64> = Vec::new();
    // Sheets whose sub-tables changed but whose scalar columns did not. A change
    // the client cannot see is only half applied: without a revision bump
    // /sync/delta would never report the sheet.
    let mut needs_revision_bump: Vec<i64> = Vec::new();
    let mut sub = SubTableColumns::default();

    for sheet in &sheets {
        // Every expr was just upserted, so it is present.
        let stored = &stored[&sheet.expr];
        if !sub_tables_differ(stored, sheet.raw) {
            continue;
        }
        dirty.push(stored.sheet_id);
        if !changed_sheets.contains_key(&sheet.expr) {
            needs_revision_bump.push(stored.sheet_id);
        }
        sub.push(stored.sheet_id, sheet.raw);
    }

    replace_sub_tables(&mut tx, &dirty, &sub).await?;

    if !needs_revision_bump.is_empty() {
        sqlx::query("UPDATE sheets SET revision = $1, updated_at = now() WHERE id = ANY($2)")
            .bind(revision)
            .bind(&needs_revision_bump)
            .execute(&mut *tx)
            .await?;
        stats.sheets_updated += needs_revision_bump.len() as i64;
    }

    // Rows the database holds that upstream no longer lists. They are logged for
    // /sync/delta and left in place — deleting a sheet would cascade to its
    // chart text. Rows already logged are not logged again: the deletion tables
    // are keyed (song_id, revision), so ON CONFLICT DO NOTHING would still write
    // a second entry under a new revision.
    stats.songs_vanished = sqlx::query_scalar::<_, i32>(
        "INSERT INTO deleted_songs (song_id, revision) \
         SELECT s.song_id, $2 FROM songs s \
         WHERE s.song_id IS NOT NULL \
           AND s.song_id <> ALL($1) \
           AND NOT EXISTS (SELECT 1 FROM deleted_songs d WHERE d.song_id = s.song_id) \
         RETURNING 1",
    )
    .bind(&song_ids)
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
    .bind(&sheet_exprs)
    .bind(revision)
    .fetch_all(&mut *tx)
    .await?
    .len() as i64;

    let (songs_vanished_total, sheets_vanished_total): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM deleted_songs), (SELECT COUNT(*) FROM deleted_sheets)",
    )
    .fetch_one(&mut *tx)
    .await?;
    stats.songs_vanished_total = songs_vanished_total;
    stats.sheets_vanished_total = sheets_vanished_total;

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

// ── lookup tables ────────────────────────────────────────────────────────────

/// md5 over each lookup table's whole contents, in `ordinal` order, in one round
/// trip. Used only to compare the tables against themselves before and after a
/// wholesale replace, so the exact row-text format does not matter — only that
/// it is stable within one transaction.
async fn lookup_digests(conn: &mut PgConnection) -> Result<Vec<String>, sqlx::Error> {
    // A table name cannot be a bind parameter. Each name is one of the five
    // literals in LOOKUP_TABLES and never reaches this function from a request
    // or from the payload, so the interpolation is audited-safe — which is
    // exactly what AssertSqlSafe asserts.
    let columns = LOOKUP_TABLES
        .iter()
        .map(|table| {
            format!(
                "(SELECT COALESCE(md5(string_agg(t::text, '|' ORDER BY t.ordinal)), '') \
                 FROM {table} t)"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT {columns}")))
        .fetch_one(conn)
        .await?;
    (0..LOOKUP_TABLES.len()).map(|i| row.try_get(i)).collect()
}

/// Ordered lookup tables carry no rows anything else references, and their
/// `ordinal` is positional, so replacing them wholesale is both safe and simpler
/// than diffing. This is the one place a DELETE is allowed.
async fn replace_lookups(conn: &mut PgConnection, data: &RawData) -> Result<(), sqlx::Error> {
    let ordinals = |n: usize| (0..n as i32).collect::<Vec<i32>>();

    sqlx::query("DELETE FROM categories")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO categories (category, ordinal) \
         SELECT * FROM unnest($1::text[], $2::int[])",
    )
    .bind(
        data.categories
            .iter()
            .map(|c| c.category.clone())
            .collect::<Vec<_>>(),
    )
    .bind(ordinals(data.categories.len()))
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM versions")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO versions (version, abbr, release_date, ordinal) \
         SELECT * FROM unnest($1::text[], $2::text[], $3::date[], $4::int[])",
    )
    .bind(
        data.versions
            .iter()
            .map(|v| v.version.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.versions
            .iter()
            .map(|v| v.abbr.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.versions
            .iter()
            .map(|v| parse_date(&v.release_date))
            .collect::<Vec<_>>(),
    )
    .bind(ordinals(data.versions.len()))
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM types").execute(&mut *conn).await?;
    sqlx::query(
        "INSERT INTO types (type, name, abbr, icon_url, icon_height, ordinal) \
         SELECT * FROM unnest($1::text[], $2::text[], $3::text[], $4::text[], \
                              $5::int[], $6::int[])",
    )
    .bind(
        data.types
            .iter()
            .map(|t| t.r#type.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.types
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.types
            .iter()
            .map(|t| t.abbr.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.types
            .iter()
            .map(|t| t.icon_url.clone())
            .collect::<Vec<_>>(),
    )
    .bind(data.types.iter().map(|t| t.icon_height).collect::<Vec<_>>())
    .bind(ordinals(data.types.len()))
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM difficulties")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO difficulties (difficulty, name, color, icon_url, icon_height, ordinal) \
         SELECT * FROM unnest($1::text[], $2::text[], $3::text[], $4::text[], \
                              $5::int[], $6::int[])",
    )
    .bind(
        data.difficulties
            .iter()
            .map(|d| d.difficulty.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.difficulties
            .iter()
            .map(|d| d.name.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.difficulties
            .iter()
            .map(|d| d.color.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.difficulties
            .iter()
            .map(|d| d.icon_url.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.difficulties
            .iter()
            .map(|d| d.icon_height)
            .collect::<Vec<_>>(),
    )
    .bind(ordinals(data.difficulties.len()))
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM regions")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO regions (region, name, ordinal) \
         SELECT * FROM unnest($1::text[], $2::text[], $3::int[])",
    )
    .bind(
        data.regions
            .iter()
            .map(|r| r.region.clone())
            .collect::<Vec<_>>(),
    )
    .bind(
        data.regions
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>(),
    )
    .bind(ordinals(data.regions.len()))
    .execute(&mut *conn)
    .await?;

    Ok(())
}

// ── songs ────────────────────────────────────────────────────────────────────

/// Payload songs that can be synced, in payload order, at most one per `songId`.
///
/// The `usize` is the song's index in `data.songs` — the source of both
/// `song_no` and `source_index`, so it must stay the *payload* position and not
/// the position in this deduplicated list.
///
/// A NULL `songId` cannot be matched on a later run (UNIQUE permits many NULLs),
/// so such a row would be re-inserted forever. None exist upstream today; they
/// are skipped and counted so it shows up in the log if that changes.
///
/// Duplicate `songId`s likewise do not exist upstream today, but they must be
/// collapsed rather than passed through: a batched `ON CONFLICT DO UPDATE`
/// fails outright with "cannot affect row a second time" on a repeated conflict
/// key, where the per-row loop this replaces absorbed it silently by letting the
/// later row update the earlier one. Keeping the last occurrence preserves that.
fn dedupe_songs<'a>(data: &'a RawData, stats: &mut SyncStats) -> Vec<(usize, &'a RawSong)> {
    let mut deduped: Vec<(usize, &RawSong)> = Vec::with_capacity(data.songs.len());
    let mut at: HashMap<&str, usize> = HashMap::with_capacity(data.songs.len());

    for (si, song) in data.songs.iter().enumerate() {
        let Some(song_id) = song.song_id.as_deref() else {
            stats.songs_skipped_no_id += 1;
            continue;
        };
        match at.get(song_id) {
            Some(&i) => deduped[i] = (si, song),
            None => {
                at.insert(song_id, deduped.len());
                deduped.push((si, song));
            }
        }
    }

    deduped
}

/// One batched upsert for every song in the payload.
///
/// Returns only the rows the statement actually wrote, keyed by `song_id`, with
/// `true` for an insert — a suppressed DO UPDATE returns nothing, which is
/// exactly what keeps `revision` still for untouched rows.
async fn upsert_songs(
    conn: &mut PgConnection,
    songs: &[(usize, &RawSong)],
    revision: i64,
) -> Result<HashMap<String, bool>, sqlx::Error> {
    // One Vec per column, all the same length: `unnest` zips them back into rows
    // server-side. Lengths must match or Postgres pads the short ones with NULL
    // instead of complaining.
    let mut song_id: Vec<String> = Vec::with_capacity(songs.len());
    let mut song_no: Vec<i32> = Vec::with_capacity(songs.len());
    let mut category: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut title: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut artist: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut bpm: Vec<Option<f64>> = Vec::with_capacity(songs.len());
    let mut image_name: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut version: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut release_date: Vec<Option<NaiveDate>> = Vec::with_capacity(songs.len());
    let mut is_new: Vec<Option<bool>> = Vec::with_capacity(songs.len());
    let mut is_locked: Vec<Option<bool>> = Vec::with_capacity(songs.len());
    let mut comment: Vec<Option<String>> = Vec::with_capacity(songs.len());
    let mut source_index: Vec<i32> = Vec::with_capacity(songs.len());

    for (si, song) in songs {
        song_id.push(
            song.song_id
                .clone()
                .expect("dedupe_songs drops NULL songIds"),
        );
        song_no.push(*si as i32 + 1);
        category.push(song.category.clone());
        title.push(song.title.clone());
        artist.push(song.artist.clone());
        bpm.push(song.bpm);
        image_name.push(song.image_name.clone());
        version.push(song.version.clone());
        release_date.push(parse_date(&song.release_date));
        is_new.push(song.is_new);
        is_locked.push(song.is_locked);
        comment.push(song.comment.clone());
        source_index.push(*si as i32);
    }

    let changed: Vec<(String, bool)> = sqlx::query_as(
        "INSERT INTO songs \
         (song_id, song_no, category, title, artist, bpm, image_name, version, \
          release_date, is_new, is_locked, comment, source_index, revision) \
         SELECT t.song_id, t.song_no, t.category, t.title, t.artist, t.bpm, t.image_name, \
                t.version, t.release_date, t.is_new, t.is_locked, t.comment, \
                t.source_index, $14::bigint \
         FROM unnest($1::text[], $2::int[], $3::text[], $4::text[], $5::text[], \
                     $6::double precision[], $7::text[], $8::text[], $9::date[], \
                     $10::bool[], $11::bool[], $12::text[], $13::int[]) \
              AS t(song_id, song_no, category, title, artist, bpm, image_name, version, \
                   release_date, is_new, is_locked, comment, source_index) \
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
         RETURNING song_id, (xmax = 0) AS inserted",
    )
    .bind(&song_id)
    .bind(&song_no)
    .bind(&category)
    .bind(&title)
    .bind(&artist)
    .bind(&bpm)
    .bind(&image_name)
    .bind(&version)
    .bind(&release_date)
    .bind(&is_new)
    .bind(&is_locked)
    .bind(&comment)
    .bind(&source_index)
    .bind(revision)
    .fetch_all(conn)
    .await?;

    Ok(changed.into_iter().collect())
}

// ── sheets ───────────────────────────────────────────────────────────────────

/// One payload sheet resolved against its parent song's primary key.
struct SheetRow<'a> {
    expr: String,
    song_pk: i64,
    /// The sheet's index within its own song — `sheets.source_index`.
    source_index: i32,
    raw: &'a RawSheet,
}

/// Every sheet of every synced song, at most one per `sheet_expr`.
///
/// Deduplicated for the same reason as [`dedupe_songs`], and on the same rule:
/// two sheets of one song sharing a `type`/`difficulty` pair collapse to the
/// later one.
fn dedupe_sheets<'a>(
    songs: &[(usize, &'a RawSong)],
    song_pks: &HashMap<String, i64>,
) -> Vec<SheetRow<'a>> {
    let mut deduped: Vec<SheetRow<'a>> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();

    for (_, song) in songs {
        let song_pk = song_pks[song
            .song_id
            .as_deref()
            .expect("dedupe_songs drops NULL songIds")];
        for (shi, sheet) in song.sheets.iter().enumerate() {
            let expr = sheet_expr(&song.song_id, &sheet.r#type, &sheet.difficulty);
            let row = SheetRow {
                expr: expr.clone(),
                song_pk,
                source_index: shi as i32,
                raw: sheet,
            };
            match at.get(&expr) {
                Some(&i) => deduped[i] = row,
                None => {
                    at.insert(expr, deduped.len());
                    deduped.push(row);
                }
            }
        }
    }

    deduped
}

/// One batched upsert for every sheet in the payload. Same contract as
/// [`upsert_songs`]: the returned map holds only rows the statement wrote.
async fn upsert_sheets(
    conn: &mut PgConnection,
    sheets: &[SheetRow<'_>],
    revision: i64,
) -> Result<HashMap<String, bool>, sqlx::Error> {
    let n = sheets.len();
    let mut song_id_fk: Vec<i64> = Vec::with_capacity(n);
    let mut expr: Vec<String> = Vec::with_capacity(n);
    let mut ty: Vec<Option<String>> = Vec::with_capacity(n);
    let mut difficulty: Vec<Option<String>> = Vec::with_capacity(n);
    let mut level: Vec<Option<String>> = Vec::with_capacity(n);
    let mut level_value: Vec<Option<f64>> = Vec::with_capacity(n);
    let mut internal_level: Vec<Option<String>> = Vec::with_capacity(n);
    let mut internal_level_value: Vec<Option<f64>> = Vec::with_capacity(n);
    let mut note_designer: Vec<Option<String>> = Vec::with_capacity(n);
    let mut is_special: Vec<Option<bool>> = Vec::with_capacity(n);
    let mut source_index: Vec<i32> = Vec::with_capacity(n);

    for sheet in sheets {
        song_id_fk.push(sheet.song_pk);
        expr.push(sheet.expr.clone());
        ty.push(sheet.raw.r#type.clone());
        difficulty.push(sheet.raw.difficulty.clone());
        level.push(sheet.raw.level.clone());
        level_value.push(sheet.raw.level_value);
        internal_level.push(sheet.raw.internal_level.clone());
        internal_level_value.push(sheet.raw.internal_level_value);
        note_designer.push(sheet.raw.note_designer.clone());
        is_special.push(sheet.raw.is_special);
        source_index.push(sheet.source_index);
    }

    let changed: Vec<(String, bool)> = sqlx::query_as(
        "INSERT INTO sheets \
         (song_id_fk, sheet_expr, type, difficulty, level, level_value, \
          internal_level, internal_level_value, note_designer, is_special, \
          source_index, revision) \
         SELECT t.song_id_fk, t.sheet_expr, t.type, t.difficulty, t.level, t.level_value, \
                t.internal_level, t.internal_level_value, t.note_designer, t.is_special, \
                t.source_index, $12::bigint \
         FROM unnest($1::bigint[], $2::text[], $3::text[], $4::text[], $5::text[], \
                     $6::double precision[], $7::text[], $8::double precision[], \
                     $9::text[], $10::bool[], $11::int[]) \
              AS t(song_id_fk, sheet_expr, type, difficulty, level, level_value, \
                   internal_level, internal_level_value, note_designer, is_special, \
                   source_index) \
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
         RETURNING sheet_expr, (xmax = 0) AS inserted",
    )
    .bind(&song_id_fk)
    .bind(&expr)
    .bind(&ty)
    .bind(&difficulty)
    .bind(&level)
    .bind(&level_value)
    .bind(&internal_level)
    .bind(&internal_level_value)
    .bind(&note_designer)
    .bind(&is_special)
    .bind(&source_index)
    .bind(revision)
    .fetch_all(conn)
    .await?;

    Ok(changed.into_iter().collect())
}

// ── sheet sub-tables ─────────────────────────────────────────────────────────

/// A sheet's three sub-tables as stored, shaped to compare directly against the
/// upstream payload.
struct StoredSubTables {
    sheet_id: i64,
    note_counts: BTreeMap<String, Option<i64>>,
    regions: BTreeMap<String, bool>,
    region_overrides: BTreeMap<String, RawOverride>,
}

/// Reads every listed sheet's id and all three of its sub-tables in **one**
/// round trip, as jsonb aggregates decoded into the same types the payload
/// parses into.
///
/// Comparing typed maps in Rust rather than digesting both sides in SQL is
/// deliberate: a digest would have to reproduce Postgres's float and NULL text
/// formatting byte-for-byte to agree, and any drift there reads as "changed"
/// forever. jsonb round-trips `double precision` through its shortest
/// round-trip representation, so `==` on the decoded `f64` is exact.
///
/// The whole payload is fetched in one statement rather than in pages: the three
/// aggregates for 7.3k sheets decode to a few MB, well within a runner, and
/// chunking would only trade that for more round trips.
async fn read_sub_tables(
    conn: &mut PgConnection,
    sheet_exprs: &[String],
) -> Result<HashMap<String, StoredSubTables>, sqlx::Error> {
    type Row = (
        String,
        i64,
        Json<BTreeMap<String, Option<i64>>>,
        Json<BTreeMap<String, bool>>,
        Json<BTreeMap<String, RawOverride>>,
    );

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.sheet_expr, s.id, \
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
         FROM sheets s WHERE s.sheet_expr = ANY($1)",
    )
    .bind(sheet_exprs)
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(expr, sheet_id, note_counts, regions, region_overrides)| {
            (
                expr,
                StoredSubTables {
                    sheet_id,
                    note_counts: note_counts.0,
                    regions: regions.0,
                    region_overrides: region_overrides.0,
                },
            )
        })
        .collect())
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

/// The rows to write back for every sheet whose sub-tables differ, as column
/// vectors for three batched inserts.
#[derive(Default)]
struct SubTableColumns {
    note_sheet_id: Vec<i64>,
    note_key: Vec<String>,
    note_value: Vec<Option<i64>>,

    region_sheet_id: Vec<i64>,
    region_region: Vec<String>,
    region_available: Vec<bool>,

    override_sheet_id: Vec<i64>,
    override_region: Vec<String>,
    override_level: Vec<Option<String>>,
    override_level_value: Vec<Option<f64>>,
    override_internal_level: Vec<Option<String>>,
    override_internal_level_value: Vec<Option<f64>>,
    override_note_designer: Vec<Option<String>>,
}

impl SubTableColumns {
    fn push(&mut self, sheet_id: i64, sheet: &RawSheet) {
        if let Some(counts) = &sheet.note_counts {
            for (key, value) in counts {
                self.note_sheet_id.push(sheet_id);
                self.note_key.push(key.clone());
                self.note_value.push(*value);
            }
        }
        if let Some(regions) = &sheet.regions {
            for (region, available) in regions {
                self.region_sheet_id.push(sheet_id);
                self.region_region.push(region.clone());
                self.region_available.push(*available);
            }
        }
        if let Some(overrides) = &sheet.region_overrides {
            for (region, ov) in overrides {
                self.override_sheet_id.push(sheet_id);
                self.override_region.push(region.clone());
                self.override_level.push(ov.level.clone());
                self.override_level_value.push(ov.level_value);
                self.override_internal_level.push(ov.internal_level.clone());
                self.override_internal_level_value
                    .push(ov.internal_level_value);
                self.override_note_designer.push(ov.note_designer.clone());
            }
        }
    }
}

/// Sub-tables key off `sheet_id` with no natural key of their own, so they are
/// replaced for every sheet whose content differs — cleared, then rewritten from
/// `columns`.
///
/// **Every DELETE here is scoped to the explicit `dirty` list of `sheet_id`s and
/// must stay that way.** A bare `DELETE FROM` on any of these three would be a
/// different statement with a very different blast radius. Scoped like this the
/// deletes cascade to nothing: `charts` hangs off `sheets`, which this module
/// never deletes.
async fn replace_sub_tables(
    conn: &mut PgConnection,
    dirty: &[i64],
    columns: &SubTableColumns,
) -> Result<(), sqlx::Error> {
    if dirty.is_empty() {
        return Ok(());
    }

    sqlx::query("DELETE FROM sheet_note_counts WHERE sheet_id = ANY($1)")
        .bind(dirty)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM sheet_regions WHERE sheet_id = ANY($1)")
        .bind(dirty)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM sheet_region_overrides WHERE sheet_id = ANY($1)")
        .bind(dirty)
        .execute(&mut *conn)
        .await?;

    sqlx::query(
        "INSERT INTO sheet_note_counts (sheet_id, key, value) \
         SELECT * FROM unnest($1::bigint[], $2::text[], $3::bigint[])",
    )
    .bind(&columns.note_sheet_id)
    .bind(&columns.note_key)
    .bind(&columns.note_value)
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "INSERT INTO sheet_regions (sheet_id, region, available) \
         SELECT * FROM unnest($1::bigint[], $2::text[], $3::bool[])",
    )
    .bind(&columns.region_sheet_id)
    .bind(&columns.region_region)
    .bind(&columns.region_available)
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "INSERT INTO sheet_region_overrides \
         (sheet_id, region, level, level_value, internal_level, \
          internal_level_value, note_designer) \
         SELECT * FROM unnest($1::bigint[], $2::text[], $3::text[], \
                              $4::double precision[], $5::text[], \
                              $6::double precision[], $7::text[])",
    )
    .bind(&columns.override_sheet_id)
    .bind(&columns.override_region)
    .bind(&columns.override_level)
    .bind(&columns.override_level_value)
    .bind(&columns.override_internal_level)
    .bind(&columns.override_internal_level_value)
    .bind(&columns.override_note_designer)
    .execute(&mut *conn)
    .await?;

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

    // ── batching ─────────────────────────────────────────────────────────────
    //
    // Every test above syncs exactly one song carrying one sheet, so a batched
    // statement is only ever exercised with a single row. `unnest` zips column
    // vectors back into rows positionally: a column listed out of order, or a
    // vector one element short, scrambles or NULLs fields *across* rows and a
    // one-row fixture cannot see it. These fixtures are deliberately wide.

    const TWO_SONGS: &str = r#"{
        "songs": [
            {
                "songId": "Alpha", "title": "Alpha Song", "artist": "A Artist",
                "bpm": 120.5, "category": "pops", "version": "maimai",
                "releaseDate": "2020-01-02", "isNew": true, "isLocked": false,
                "comment": "first",
                "sheets": [
                    { "type": "std", "difficulty": "expert", "level": "12",
                      "levelValue": 12.0, "internalLevel": "12.3",
                      "internalLevelValue": 12.3, "noteDesigner": "Designer One",
                      "isSpecial": false,
                      "noteCounts": { "tap": 1 }, "regions": { "jp": true } },
                    { "type": "dx", "difficulty": "master", "level": "13",
                      "levelValue": 13.0, "noteDesigner": "Designer Two",
                      "noteCounts": { "tap": 2 }, "regions": { "jp": true } }
                ]
            },
            {
                "songId": "Beta", "title": "Beta Song", "artist": "B Artist",
                "bpm": 200.0, "category": "niconico", "isNew": false,
                "sheets": [
                    { "type": "dx", "difficulty": "master", "level": "14+",
                      "levelValue": 14.7, "noteDesigner": "Designer Three",
                      "noteCounts": { "tap": 3 }, "regions": { "intl": false } }
                ]
            }
        ]
    }"#;

    // A scrambled column vector still inserts the right *number* of rows, so the
    // assertion that matters is that every field landed on its own row.
    #[sqlx::test]
    async fn a_multi_row_batch_keeps_each_field_on_its_own_row(pool: PgPool) -> sqlx::Result<()> {
        let stats = apply(&pool, &payload(TWO_SONGS)).await?;
        assert_eq!(stats.songs_inserted, 2);
        assert_eq!(stats.sheets_inserted, 3);

        type SongRow = (
            String,
            Option<String>,
            Option<String>,
            Option<f64>,
            Option<String>,
            Option<bool>,
            Option<String>,
            Option<i32>,
            i32,
        );
        let songs: Vec<SongRow> = sqlx::query_as(
            "SELECT song_id, title, artist, bpm, category, is_new, \
                    to_char(release_date, 'YYYY-MM-DD'), song_no, source_index \
             FROM songs ORDER BY source_index",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            songs,
            vec![
                (
                    "Alpha".into(),
                    Some("Alpha Song".into()),
                    Some("A Artist".into()),
                    Some(120.5),
                    Some("pops".into()),
                    Some(true),
                    Some("2020-01-02".into()),
                    Some(1),
                    0
                ),
                (
                    "Beta".into(),
                    Some("Beta Song".into()),
                    Some("B Artist".into()),
                    Some(200.0),
                    Some("niconico".into()),
                    Some(false),
                    None,
                    Some(2),
                    1
                ),
            ]
        );

        // song_id_fk is resolved in Rust from a map, so a sheet attached to the
        // wrong parent is the other way a batch can go quietly wrong.
        type SheetRow = (
            String,
            String,
            Option<String>,
            Option<f64>,
            Option<String>,
            i32,
        );
        let sheets: Vec<SheetRow> = sqlx::query_as(
            "SELECT sh.sheet_expr, s.song_id, sh.level, sh.level_value, \
                    sh.note_designer, sh.source_index \
             FROM sheets sh JOIN songs s ON s.id = sh.song_id_fk \
             ORDER BY s.source_index, sh.source_index",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            sheets,
            vec![
                (
                    "Alpha|std|expert".into(),
                    "Alpha".into(),
                    Some("12".into()),
                    Some(12.0),
                    Some("Designer One".into()),
                    0
                ),
                (
                    "Alpha|dx|master".into(),
                    "Alpha".into(),
                    Some("13".into()),
                    Some(13.0),
                    Some("Designer Two".into()),
                    1
                ),
                (
                    "Beta|dx|master".into(),
                    "Beta".into(),
                    Some("14+".into()),
                    Some(14.7),
                    Some("Designer Three".into()),
                    0
                ),
            ]
        );

        // Sub-tables are batched across every dirty sheet at once, so they carry
        // the same misalignment risk.
        let counts: Vec<(String, String, Option<i32>)> = sqlx::query_as(
            "SELECT sh.sheet_expr, n.key, n.value \
             FROM sheet_note_counts n JOIN sheets sh ON sh.id = n.sheet_id \
             ORDER BY sh.sheet_expr, n.key",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            counts,
            vec![
                ("Alpha|dx|master".into(), "tap".into(), Some(2)),
                ("Alpha|std|expert".into(), "tap".into(), Some(1)),
                ("Beta|dx|master".into(), "tap".into(), Some(3)),
            ]
        );

        let regions: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT sh.sheet_expr, r.region, r.available \
             FROM sheet_regions r JOIN sheets sh ON sh.id = r.sheet_id \
             ORDER BY sh.sheet_expr",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            regions,
            vec![
                ("Alpha|dx|master".into(), "jp".into(), true),
                ("Alpha|std|expert".into(), "jp".into(), true),
                ("Beta|dx|master".into(), "intl".into(), false),
            ]
        );

        Ok(())
    }

    #[sqlx::test]
    async fn a_multi_row_batch_rerun_bumps_nothing(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(TWO_SONGS)).await?;
        let stats = apply(&pool, &payload(TWO_SONGS)).await?;

        assert_eq!(stats.songs_updated, 0);
        assert_eq!(stats.sheets_updated, 0);
        assert!(!stats.revision_advanced);
        assert_eq!(catalog_meta_revision(&pool).await, 1);
        Ok(())
    }

    // `song_no` and `source_index` come from the song's position in the *payload*,
    // not its position in the deduplicated list the batch is built from. A skipped
    // song in the middle is what separates the two.
    #[sqlx::test]
    async fn a_skipped_song_does_not_shift_later_source_indexes(pool: PgPool) -> sqlx::Result<()> {
        let data = payload(
            r#"{
                "songs": [
                    { "title": "No Id", "sheets": [] },
                    { "songId": "Alpha", "title": "Alpha", "sheets": [] },
                    { "songId": "Beta", "title": "Beta", "sheets": [] }
                ]
            }"#,
        );
        let stats = apply(&pool, &data).await?;
        assert_eq!(stats.songs_skipped_no_id, 1);
        assert_eq!(stats.songs_inserted, 2);

        let rows: Vec<(String, Option<i32>, i32)> = sqlx::query_as(
            "SELECT song_id, song_no, source_index FROM songs ORDER BY source_index",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            rows,
            vec![
                ("Alpha".to_string(), Some(2), 1),
                ("Beta".to_string(), Some(3), 2),
            ]
        );
        Ok(())
    }

    // A batched `ON CONFLICT DO UPDATE` aborts the whole statement with "cannot
    // affect row a second time" if one conflict key appears twice, where the
    // per-row loop this replaced absorbed a duplicate by letting the later row
    // update the earlier one. Upstream carries no duplicates today, so without
    // the dedupe the first one to appear would fail the nightly sync outright.
    #[sqlx::test]
    async fn duplicate_song_ids_collapse_to_the_last_rather_than_failing(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let data = payload(
            r#"{
                "songs": [
                    { "songId": "Alpha", "title": "First", "sheets": [] },
                    { "songId": "Alpha", "title": "Second", "sheets": [] }
                ]
            }"#,
        );
        let stats = apply(&pool, &data).await?;

        assert_eq!(stats.songs_inserted, 1);
        assert_eq!(stats.songs_updated, 0);

        let rows: Vec<(String, Option<String>, i32)> =
            sqlx::query_as("SELECT song_id, title, source_index FROM songs")
                .fetch_all(&pool)
                .await?;
        assert_eq!(
            rows,
            vec![("Alpha".to_string(), Some("Second".to_string()), 1)]
        );
        Ok(())
    }

    #[sqlx::test]
    async fn duplicate_sheet_exprs_collapse_to_the_last_rather_than_failing(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let data = payload(
            r#"{
                "songs": [{
                    "songId": "Alpha", "title": "Alpha",
                    "sheets": [
                        { "type": "dx", "difficulty": "master", "level": "13" },
                        { "type": "dx", "difficulty": "master", "level": "14" }
                    ]
                }]
            }"#,
        );
        let stats = apply(&pool, &data).await?;

        assert_eq!(stats.sheets_inserted, 1);
        assert_eq!(stats.sheets_updated, 0);

        let rows: Vec<(String, Option<String>, i32)> =
            sqlx::query_as("SELECT sheet_expr, level, source_index FROM sheets")
                .fetch_all(&pool)
                .await?;
        assert_eq!(
            rows,
            vec![("Alpha|dx|master".to_string(), Some("14".to_string()), 1)]
        );
        Ok(())
    }

    // The sub-table DELETEs are now one statement scoped to a list of sheet_ids
    // rather than one statement per sheet. Widening that list to every sheet in
    // the payload would still leave correct *content* behind — it is rewritten
    // immediately — so the observable damage would be revision churn on sheets
    // that never changed, which is exactly what /sync/delta keys off.
    #[sqlx::test]
    async fn a_sub_table_change_bumps_only_its_own_sheet(pool: PgPool) -> sqlx::Result<()> {
        apply(&pool, &payload(TWO_SONGS)).await?;

        // Only Alpha|std|expert's noteCounts differ; every other sheet, and every
        // scalar column everywhere, is byte-identical.
        let backfilled = TWO_SONGS.replace(
            r#""noteCounts": { "tap": 1 }"#,
            r#""noteCounts": { "tap": 9 }"#,
        );
        let stats = apply(&pool, &payload(&backfilled)).await?;

        assert_eq!(stats.sheets_updated, 1);
        assert_eq!(stats.songs_updated, 0);

        let revisions: Vec<(String, i64)> =
            sqlx::query_as("SELECT sheet_expr, revision FROM sheets ORDER BY sheet_expr")
                .fetch_all(&pool)
                .await?;
        assert_eq!(
            revisions,
            vec![
                ("Alpha|dx|master".to_string(), 1),
                ("Alpha|std|expert".to_string(), 2),
                ("Beta|dx|master".to_string(), 1),
            ],
            "an untouched sheet must not be swept up by the batched delete"
        );

        // And the untouched sheets' sub-table rows must still be there.
        let counts: Vec<(String, Option<i32>)> = sqlx::query_as(
            "SELECT sh.sheet_expr, n.value \
             FROM sheet_note_counts n JOIN sheets sh ON sh.id = n.sheet_id \
             ORDER BY sh.sheet_expr",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            counts,
            vec![
                ("Alpha|dx|master".to_string(), Some(2)),
                ("Alpha|std|expert".to_string(), Some(9)),
                ("Beta|dx|master".to_string(), Some(3)),
            ]
        );
        Ok(())
    }

    // An empty payload binds empty arrays to every parameter. Postgres needs the
    // explicit `::type[]` casts on `unnest` to infer anything at all from those,
    // so this is the test that fails if one is dropped.
    #[sqlx::test]
    async fn an_empty_payload_binds_empty_arrays_without_erroring(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let stats = apply(&pool, &payload(r#"{ "songs": [] }"#)).await?;
        assert_eq!(stats.songs_inserted, 0);
        assert_eq!(stats.sheets_inserted, 0);
        assert!(!stats.revision_advanced);
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
