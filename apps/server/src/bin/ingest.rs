//! Ingest: fetch the upstream `data.json` and load it into the canonical tables.
//!
//!   cargo run --bin ingest
//!
//! Idempotent: truncates the canonical tables then reinserts, so re-running
//! refreshes the catalog from scratch. Reads DATABASE_URL from .env.
//! Runtime queries (not the `query!` macro) so it builds with no DB present.

use std::collections::BTreeMap;
use std::error::Error;

use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::{DateTime, NaiveDate, Utc};

const DATA_URL: &str = "https://dp4p6x0xfi5o9.cloudfront.net/maimai/data.json";

// ── upstream data.json shape (raw fields; mirrors types/{Data,Song,Sheet}.ts) ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawData {
    #[serde(default)]
    songs: Vec<RawSong>,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    versions: Vec<RawVersion>,
    #[serde(default)]
    types: Vec<RawType>,
    #[serde(default)]
    difficulties: Vec<RawDifficulty>,
    #[serde(default)]
    regions: Vec<RawRegion>,
    update_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCategory {
    category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVersion {
    version: String,
    abbr: Option<String>,
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawType {
    r#type: String,
    name: String,
    abbr: Option<String>,
    icon_url: Option<String>,
    icon_height: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDifficulty {
    difficulty: String,
    name: String,
    color: Option<String>,
    icon_url: Option<String>,
    icon_height: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RawRegion {
    region: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSong {
    song_id: Option<String>,
    category: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    bpm: Option<f64>,
    image_name: Option<String>,
    version: Option<String>,
    release_date: Option<String>,
    is_new: Option<bool>,
    is_locked: Option<bool>,
    comment: Option<String>,
    #[serde(default)]
    sheets: Vec<RawSheet>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSheet {
    r#type: Option<String>,
    difficulty: Option<String>,
    level: Option<String>,
    level_value: Option<f64>,
    internal_level: Option<String>,
    internal_level_value: Option<f64>,
    note_designer: Option<String>,
    note_counts: Option<BTreeMap<String, Option<i64>>>,
    regions: Option<BTreeMap<String, bool>>,
    region_overrides: Option<BTreeMap<String, RawOverride>>,
    is_special: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOverride {
    level: Option<String>,
    level_value: Option<f64>,
    internal_level: Option<String>,
    internal_level_value: Option<f64>,
    note_designer: Option<String>,
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn parse_date(s: &Option<String>) -> Option<NaiveDate> {
    s.as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

// sheet_expr = songId|type|difficulty (mirrors utils/sheet.ts:computeSheetExpr,
// where a null/undefined part stringifies to its JS literal).
fn sheet_expr(song_id: &Option<String>, ty: &Option<String>, diff: &Option<String>) -> String {
    format!(
        "{}|{}|{}",
        song_id.as_deref().unwrap_or("null"),
        ty.as_deref().unwrap_or("undefined"),
        diff.as_deref().unwrap_or("undefined"),
    )
}

// ── main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;

    let body = reqwest::get(DATA_URL).await?.text().await?;
    let data: RawData = serde_json::from_str(&body)?;
    println!(
        "fetched: {} songs, {} categories, {} versions",
        data.songs.len(),
        data.categories.len(),
        data.versions.len(),
    );

    ingest(&pool, &data).await?;

    let total_sheets: i64 = data.songs.iter().map(|s| s.sheets.len() as i64).sum();
    println!("ingested {} songs, {} sheets", data.songs.len(), total_sheets);
    Ok(())
}

async fn ingest(pool: &PgPool, data: &RawData) -> Result<(), Box<dyn Error>> {
    let mut tx = pool.begin().await?;

    // Every full reload gets a fresh revision, and marks itself as the point
    // GET /sync/delta can't diff across (see the sync-tier plan). Read the
    // previous revision from the about-to-be-truncated catalog_meta first.
    let previous_revision: i64 = sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0);
    let new_revision = previous_revision + 1;

    // Wipe canonical tables; CASCADE clears sheets + sub-tables via FKs.
    sqlx::query(
        "TRUNCATE catalog_meta, categories, versions, types, difficulties, regions, songs \
         RESTART IDENTITY CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    // catalog_meta (singleton): Data.updateTime → midnight UTC, or now() fallback.
    let update_time: DateTime<Utc> = parse_date(&data.update_time)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(Utc::now);
    sqlx::query(
        "INSERT INTO catalog_meta (id, update_time, revision, last_full_reload_revision) \
         VALUES (true, $1, $2, $2)",
    )
    .bind(update_time)
    .bind(new_revision)
    .execute(&mut *tx)
    .await?;

    // Ordered lookup tables — ordinal = array index.
    for (i, c) in data.categories.iter().enumerate() {
        sqlx::query("INSERT INTO categories (category, ordinal) VALUES ($1, $2)")
            .bind(&c.category)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
    }
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
    for (i, r) in data.regions.iter().enumerate() {
        sqlx::query("INSERT INTO regions (region, name, ordinal) VALUES ($1, $2, $3)")
            .bind(&r.region)
            .bind(&r.name)
            .bind(i as i32)
            .execute(&mut *tx)
            .await?;
    }

    // Songs → sheets → sub-tables. source_index = ingest order.
    for (si, song) in data.songs.iter().enumerate() {
        let song_pk: i64 = sqlx::query_scalar(
            "INSERT INTO songs \
             (song_id, song_no, category, title, artist, bpm, image_name, version, \
              release_date, is_new, is_locked, comment, source_index, revision) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id",
        )
        .bind(&song.song_id)
        .bind(si as i32 + 1) // song_no; client recomputes anyway
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
        .bind(new_revision)
        .fetch_one(&mut *tx)
        .await?;

        for (shi, sheet) in song.sheets.iter().enumerate() {
            let expr = sheet_expr(&song.song_id, &sheet.r#type, &sheet.difficulty);
            let sheet_pk: i64 = sqlx::query_scalar(
                "INSERT INTO sheets \
                 (song_id_fk, sheet_expr, type, difficulty, level, level_value, \
                  internal_level, internal_level_value, note_designer, is_special, source_index, revision) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
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
            .bind(new_revision)
            .fetch_one(&mut *tx)
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

    tx.commit().await?;
    Ok(())
}
