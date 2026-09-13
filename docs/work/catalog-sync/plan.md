# Differential Catalog Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `bin/ingest`'s truncate-and-reload with a scheduled differential sync that never deletes, so chart text survives catalog refreshes and `/sync/delta` keeps working across them.

**Architecture:** Upstream JSON types move from `bin/ingest.rs` into a library module so both the binary and its tests can use them. A second library module holds `apply()`, which takes an already-parsed `RawData` and a transaction and performs the diff — that seam is what makes the whole thing testable without network access. A thin binary does fetch, sanity-gate, call `apply`, log. Sub-tables hanging off `sheet_id` are replaced per sheet; `songs`, `sheets` and `charts` rows are never deleted.

**Tech Stack:** Rust 1.97 (edition 2024), sqlx 0.9 with Postgres, reqwest 0.13, tokio, serde. Tests are `#[sqlx::test]`, which provisions a throwaway database per test and applies `migrations/` automatically.

**Spec:** [`docs/work/catalog-sync/spec.md`](spec.md)

## Global Constraints

- **Runtime queries only** — `sqlx::query(...)`, never `sqlx::query!(...)`. The macros write entries into the committed `.sqlx/` cache, which then has to be regenerated with `cargo sqlx prepare --workspace -- -p server --all-targets`. Following `bin/ingest.rs`'s existing choice keeps this feature out of the cache entirely.
- **No `DELETE` against `songs`, `sheets` or `charts`.** Ever. `sheets.song_id_fk` and `charts.sheet_id` are both `ON DELETE CASCADE`, so a delete on either parent destroys chart text. Deleting from `sheet_note_counts`, `sheet_regions` and `sheet_region_overrides` is fine — nothing cascades from them.
- **Never write `catalog_meta.last_full_reload_revision`.** It must keep its existing value forever; that is what makes `/sync/delta` survive a refresh.
- **A row's `revision` advances only when a field actually changed.** Use `ON CONFLICT ... DO UPDATE ... WHERE` with a `ROW(...) IS DISTINCT FROM ROW(...)` guard.
- Upstream URL: `https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json`
- Sanity gate: incoming `songs` must be non-empty, and its length ≥ 90% of the current `COUNT(*)` from `songs`, with the ratio check skipped when that count is 0.
- Every task ends green: `cargo fmt --check`, `cargo clippy -p server --all-targets -- -D warnings`, and `cargo test -p server` all pass before the commit.

---

### Task 1: Move upstream types into a library module

`bin/ingest.rs` will be deleted in Task 6, and its `Raw*` types are private to that binary. They move first so nothing else depends on the file.

**Files:**
- Create: `apps/server/src/upstream.rs`
- Modify: `apps/server/src/lib.rs`
- Modify: `apps/server/src/bin/ingest.rs:19-138` (delete the moved definitions, import instead)

**Interfaces:**
- Consumes: nothing
- Produces: `server::upstream::{RawData, RawCategory, RawVersion, RawType, RawDifficulty, RawRegion, RawSong, RawSheet, RawOverride}`, plus `server::upstream::parse_date(&Option<String>) -> Option<chrono::NaiveDate>` and `server::upstream::sheet_expr(&Option<String>, &Option<String>, &Option<String>) -> String`

- [ ] **Step 1: Write the failing test**

Create `apps/server/src/upstream.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimal_payload() {
        let json = r#"{
            "updateTime": "2026-09-11",
            "songs": [{
                "songId": "Example",
                "title": "Example",
                "sheets": [{ "type": "dx", "difficulty": "master", "level": "14+" }]
            }],
            "categories": [{ "category": "pops" }]
        }"#;

        let data: RawData = serde_json::from_str(json).unwrap();

        assert_eq!(data.songs.len(), 1);
        assert_eq!(data.songs[0].song_id.as_deref(), Some("Example"));
        assert_eq!(data.songs[0].sheets.len(), 1);
        assert_eq!(data.categories.len(), 1);
        // Absent arrays default to empty rather than failing to parse.
        assert!(data.regions.is_empty());
    }

    #[test]
    fn sheet_expr_uses_js_literals_for_missing_parts() {
        assert_eq!(
            sheet_expr(&Some("Example".into()), &Some("dx".into()), &Some("master".into())),
            "Example|dx|master"
        );
        assert_eq!(sheet_expr(&None, &None, &None), "null|undefined|undefined");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p server --lib upstream`
Expected: FAIL — `cannot find type RawData in this scope`, `cannot find function sheet_expr`.

- [ ] **Step 3: Move the definitions**

Cut lines 19–138 of `apps/server/src/bin/ingest.rs` (the `// ── upstream data.json shape` block through `fn sheet_expr`) and paste them **above** the test module in `apps/server/src/upstream.rs`. Make every item and every field `pub`, and add the module docs:

```rust
//! The upstream `data.json` shape, mirroring `apps/host/src/types/{Data,Song,Sheet}.ts`.
//!
//! Lives in the library rather than in a binary because `catalog_sync` and its
//! tests both need it — a `src/bin/*.rs` file is its own crate root and cannot
//! be imported.

use serde::Deserialize;
use sqlx::types::chrono::NaiveDate;
use std::collections::BTreeMap;
```

Every struct keeps its existing derives and `#[serde(rename_all = "camelCase")]` attributes exactly as they are. `parse_date` and `sheet_expr` become `pub fn`.

Then add to `apps/server/src/lib.rs`:

```rust
pub mod upstream;
```

And in `apps/server/src/bin/ingest.rs`, replace the deleted block with:

```rust
use server::upstream::{RawData, parse_date, sheet_expr};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p server --lib upstream`
Expected: PASS, 2 tests.

Run: `cargo build -p server --bins`
Expected: success — `ingest` still compiles against the moved types.

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/upstream.rs apps/server/src/lib.rs apps/server/src/bin/ingest.rs
git commit -m "refactor(server): move upstream data.json types into a library module"
```

---

### Task 2: `apply()` writes the lookup tables and bumps the revision

The first slice of the diff engine: `catalog_meta` and the five ordered lookup tables. Songs and sheets come in Task 3.

**Files:**
- Create: `apps/server/src/catalog_sync.rs`
- Modify: `apps/server/src/lib.rs`

**Interfaces:**
- Consumes: `server::upstream::{RawData, parse_date}`
- Produces:
  - `pub struct SyncStats { pub songs_inserted: i64, pub songs_updated: i64, pub sheets_inserted: i64, pub sheets_updated: i64, pub songs_vanished: i64, pub sheets_vanished: i64, pub songs_skipped_no_id: i64, pub revision: i64 }`
  - `pub async fn apply(pool: &sqlx::PgPool, data: &RawData) -> Result<SyncStats, sqlx::Error>`

- [ ] **Step 1: Write the failing test**

Create `apps/server/src/catalog_sync.rs` with:

```rust
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

        let (revision, last_full): (i64, i64) = sqlx::query_as(
            "SELECT revision, last_full_reload_revision FROM catalog_meta",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(revision, 1);
        // Never advanced — this is what keeps /sync/delta working across syncs.
        assert_eq!(last_full, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn second_sync_advances_the_revision_and_replaces_lookups(pool: PgPool) -> sqlx::Result<()> {
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p server --lib catalog_sync`
Expected: FAIL — `cannot find function apply in this scope`.

- [ ] **Step 3: Write the implementation**

Above the test module in `apps/server/src/catalog_sync.rs`:

```rust
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
    sqlx::query("DELETE FROM categories").execute(&mut *tx).await?;
    for (i, c) in data.categories.iter().enumerate() {
        sqlx::query("INSERT INTO categories (category, ordinal) VALUES ($1, $2)")
            .bind(&c.category)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query("DELETE FROM versions").execute(&mut *tx).await?;
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

    sqlx::query("DELETE FROM difficulties").execute(&mut *tx).await?;
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
```

Add to `apps/server/src/lib.rs`:

```rust
pub mod catalog_sync;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p server --lib catalog_sync`
Expected: PASS, 2 tests. (Requires Postgres: `docker compose up -d` from the repo root.)

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/catalog_sync.rs apps/server/src/lib.rs
git commit -m "feat(server): catalog_sync applies meta and lookup tables"
```

---

### Task 3: Upsert songs and sheets, bumping revision only on real change

The heart of the feature. A row untouched upstream must come out of a sync with its `revision` unchanged, or `/sync/delta` degenerates into "everything changed".

**Files:**
- Modify: `apps/server/src/catalog_sync.rs`

**Interfaces:**
- Consumes: `SyncStats` and `apply` from Task 2; `server::upstream::sheet_expr`
- Produces: `apply` now populates `songs_inserted`, `songs_updated`, `sheets_inserted`, `sheets_updated`, `songs_skipped_no_id`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `apps/server/src/catalog_sync.rs`:

```rust
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
        assert_eq!(stats.sheets_updated, 0, "unchanged sheet must not be updated");

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
        sqlx::query(
            "INSERT INTO charts (sheet_id, sheet_expr, content) VALUES ($1, $2, $3)",
        )
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
        assert_eq!(charts, 1, "chart text must survive a song vanishing upstream");

        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p server --lib catalog_sync`
Expected: FAIL — the new assertions fail because `apply` does not touch `songs` or `sheets` yet (`songs_inserted` is 0, and the `songs` table stays empty).

- [ ] **Step 3: Write the implementation**

In `apps/server/src/catalog_sync.rs`, add `use crate::upstream::sheet_expr;` to the imports, and insert this **before** `tx.commit()`:

```rust
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
```

Note on `xmax = 0`: Postgres exposes it on the returned row, and it is `0` exactly when the row was newly inserted rather than updated. That is how insert and update are told apart in one statement.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p server --lib catalog_sync`
Expected: PASS, 7 tests.

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/catalog_sync.rs
git commit -m "feat(server): upsert songs and sheets, bumping revision only on change"
```

---

### Task 4: Record rows that vanished upstream

**Files:**
- Modify: `apps/server/src/catalog_sync.rs`

**Interfaces:**
- Consumes: `apply`, `SyncStats` from Task 3
- Produces: `apply` now populates `songs_vanished` and `sheets_vanished`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module:

```rust
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

        assert_eq!(stats.songs_vanished, 0, "already logged on the previous sync");

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p server --lib catalog_sync`
Expected: FAIL — `songs_vanished` is 0 and `deleted_songs` is empty.

- [ ] **Step 3: Write the implementation**

Insert **before** `tx.commit()` in `apply`, after the songs loop. `data.songs` is borrowed, so build the present-key sets first:

```rust
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
```

The `fetch_all(...).len()` counts rows actually written; `RETURNING 1` gives each a value to count. Note `x <> ALL('{}')` is **true** — a comparison against every element of an empty set holds vacuously — so an empty upstream payload marks every row vanished, which is what the first test above pins.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p server --lib catalog_sync`
Expected: PASS, 10 tests.

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/catalog_sync.rs
git commit -m "feat(server): log rows that vanished upstream without deleting them"
```

---

### Task 5: The `sync_catalog` binary — fetch and sanity gate

**Files:**
- Create: `apps/server/src/bin/sync_catalog.rs`
- Modify: `apps/server/src/catalog_sync.rs` (add `sanity_check`)

**Interfaces:**
- Consumes: `server::catalog_sync::{apply, SyncStats}`, `server::upstream::RawData`
- Produces: `pub fn sanity_check(incoming: usize, current: i64) -> Result<(), String>` in `catalog_sync`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `apps/server/src/catalog_sync.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p server --lib catalog_sync::tests::sanity`
Expected: FAIL — `cannot find function sanity_check in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `apps/server/src/catalog_sync.rs`, above the tests:

```rust
/// Refuse implausible payloads before any write. The failure this guards is a
/// truncated or malformed upstream response landing in production unattended —
/// see docs/work/catalog-sync/spec.md.
pub fn sanity_check(incoming: usize, current: i64) -> Result<(), String> {
    if incoming == 0 {
        return Err("upstream returned zero songs".to_string());
    }
    if current > 0 {
        let floor = (current as f64 * 0.9).floor() as usize;
        if incoming < floor {
            return Err(format!(
                "upstream returned {incoming} songs, below the {floor} floor (90% of {current})"
            ));
        }
    }
    Ok(())
}
```

Create `apps/server/src/bin/sync_catalog.rs`:

```rust
//! Differential catalog sync: fetch upstream, sanity-check, apply the diff.
//!
//!   cargo run --bin sync_catalog
//!
//! Runs daily from .github/workflows/sync-catalog.yml. Unlike the `ingest` it
//! replaced, this never truncates and never deletes a song or sheet, so chart
//! text survives — see docs/work/catalog-sync/spec.md.

use std::error::Error;
use std::time::Duration;

use server::catalog_sync::{apply, sanity_check};
use server::upstream::RawData;
use sqlx::postgres::PgPoolOptions;

const DATA_URL: &str = "https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json";

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("sync_catalog: failed: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;

    println!("sync_catalog: fetching {DATA_URL}");
    let body = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?
        .get(DATA_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let data: RawData = serde_json::from_str(&body)?;
    println!("sync_catalog: parsed {} songs", data.songs.len());

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    let current: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs")
        .fetch_one(&pool)
        .await?;
    sanity_check(data.songs.len(), current)?;

    let stats = apply(&pool, &data).await?;
    println!(
        "sync_catalog: revision {} — songs +{} ~{} vanished {} skipped {}, sheets +{} ~{} vanished {}",
        stats.revision,
        stats.songs_inserted,
        stats.songs_updated,
        stats.songs_vanished,
        stats.songs_skipped_no_id,
        stats.sheets_inserted,
        stats.sheets_updated,
        stats.sheets_vanished,
    );

    Ok(())
}
```

- [ ] **Step 4: Run tests and exercise the binary**

Run: `cargo test -p server --lib catalog_sync`
Expected: PASS, 14 tests.

Run against the local database (`docker compose up -d` from the repo root, then from `apps/server`):
```bash
cargo run --bin sync_catalog
cargo run --bin sync_catalog   # second run
```
Expected: the first run reports non-zero inserts; the second reports `songs +0 ~0` — the no-op case that proves revisions stay still.

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/bin/sync_catalog.rs apps/server/src/catalog_sync.rs
git commit -m "feat(server): bin/sync_catalog fetches, gates and applies the diff"
```

---

### Task 6: Delete `bin/ingest`

**Files:**
- Delete: `apps/server/src/bin/ingest.rs`
- Modify: `CLAUDE.md:30-36` (the Backend commands block)
- Modify: `docs/guides/local-setup.md:15-24`
- Modify: `apps/server/Dockerfile` — only if it names `ingest`; it should not, since it builds `--bin server --bin migrate`

**Interfaces:**
- Consumes: nothing
- Produces: nothing

- [ ] **Step 1: Confirm nothing references it**

Run:
```bash
grep -rn "bin/ingest\|--bin ingest\|bin ingest" --include="*.rs" --include="*.toml" --include="*.yml" --include="*.md" --include="Dockerfile" . | grep -v "^./target" | grep -v "docs/work/"
```
Expected: hits only in `CLAUDE.md` and `docs/guides/local-setup.md`. Any hit in `.github/workflows/` or `apps/server/Dockerfile` must be fixed in this task.

- [ ] **Step 2: Delete and update the docs**

```bash
git rm apps/server/src/bin/ingest.rs
```

In `CLAUDE.md`, replace the `cargo run --bin ingest` line with:

```sh
cargo run --bin sync_catalog       # fetch upstream and apply the diff
```

In `docs/guides/local-setup.md`, replace `cargo run --bin ingest        # load the catalog` with:

```sh
cargo run --bin sync_catalog  # fetch upstream and apply the diff
```

- [ ] **Step 3: Verify the workspace still builds**

Run: `cargo build -p server --bins && cargo test -p server`
Expected: PASS. `ingest` no longer appears in the binary list.

- [ ] **Step 4: Verify the offline cache is unaffected**

Run:
```bash
set -a; . apps/server/.env; set +a
cargo sqlx prepare --check --workspace -- -p server --all-targets
```
Expected: exit 0. `ingest` used runtime queries, and so does `catalog_sync`, so `.sqlx/` should not change. **If this fails**, regenerate with the same flags minus `--check` and commit the result.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(server)!: delete bin/ingest, superseded by sync_catalog

TRUNCATE songs CASCADE reached sheets and then charts, so every catalog
refresh destroyed all chart text. sync_catalog never deletes."
```

---

### Task 7: Daily scheduled workflow

**Files:**
- Create: `.github/workflows/sync-catalog.yml`
- Modify: `docs/work/catalog-sync/spec.md` (tick the scope table)

**Interfaces:**
- Consumes: `cargo run --bin sync_catalog` from Task 5
- Produces: nothing

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/sync-catalog.yml`:

```yaml
name: sync-catalog

# Daily differential sync of the upstream catalog. Safe to run unattended
# because bin/sync_catalog never deletes and refuses implausible payloads
# before writing — see docs/work/catalog-sync/spec.md.
on:
  schedule:
    # 19:00 UTC = 02:00 Asia/Ho_Chi_Minh, off-peak for the primary region.
    - cron: "0 19 * * *"
  workflow_dispatch:

concurrency:
  group: sync-catalog
  cancel-in-progress: false

jobs:
  sync:
    runs-on: ubuntu-latest
    environment: production

    steps:
      - uses: actions/checkout@v4

      - name: Install the pinned toolchain
        run: rustup show

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: native

      - name: Run the sync
        env:
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
        run: cargo run --release --bin sync_catalog
```

- [ ] **Step 2: Validate the workflow parses**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/sync-catalog.yml')); print('ok')"`
Expected: `ok`

- [ ] **Step 3: Add the required secret**

`DATABASE_URL` must exist on the `production` environment in GitHub (Settings → Environments → production). Use Neon's **direct** endpoint, not the pooler: `apply` runs inside a transaction, and transaction-mode pooling does not guarantee one backend across statements.

This is a manual step; the workflow fails with `DATABASE_URL is not set` until it is done.

- [ ] **Step 4: Dispatch once and verify**

Trigger `sync-catalog` manually from the Actions tab. Expected in the log:

```
sync_catalog: fetching https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json
sync_catalog: parsed 1845 songs
sync_catalog: revision N — songs +... ~... vanished 0 skipped 0, sheets +... ~... vanished 0
```

Then confirm the API serves it: `curl -s https://maiscope-api.fly.dev/api/v1/catalog | head -c 200` should return catalog JSON rather than `{"error":"database_error"}`.

Dispatch a second time; expect `songs +0 ~0` unless upstream changed between runs.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/sync-catalog.yml docs/work/catalog-sync/spec.md
git commit -m "feat(ci): daily catalog sync workflow"
```

---

## Follow-ups this plan deliberately leaves open

- **`prod-data-and-infra` 02 and 03** assume truncate-and-reload and need amending, not deleting — a vendored snapshot still has value as a backup even when rows are applied differentially.
- **`prod-data-and-infra` 06** (`/catalog` 500s on an empty database) is *masked* once a sync has run, not fixed. The `fetch_one` on `catalog_meta` is still wrong for a fresh environment.
- **Vanished rows stay visible in `/catalog`** — the accepted cost recorded in the spec. Revisit if drift becomes observable.
