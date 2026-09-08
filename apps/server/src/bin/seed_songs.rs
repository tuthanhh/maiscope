//! Bulk-seed charts from local maidata.txt files (one per song directory under
//! `songs/`, e.g. `songs/larva/maidata.txt`) into the DB, keyed by sheet_expr.
//! Same upsert target as seed_chart, batched across every difficulty a
//! maidata file provides.
//!
//!   cargo run --bin seed_songs [-- <path/to/songs-dir>]
//!
//! Matches each maidata's &title= against the catalog's songs.title (NFC-
//! normalized, since upstream data can use a different but visually
//! identical Unicode form — e.g. U+212B ANGSTROM SIGN vs U+00C5 Å). A
//! difficulty present in the maidata with no matching sheet in the catalog
//! is a warning (song legitimately doesn't have that difficulty, or the
//! catalog is stale) — except master/remaster, which are commonly and
//! expectedly absent for many songs, so those are skipped silently.

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use unicode_normalization::UnicodeNormalization;

/// maidata inote slot -> this app's difficulty code (mirrors sheets.difficulty).
const SLOT_DIFFICULTIES: &[(&str, &str)] = &[
    ("2", "basic"),
    ("3", "advanced"),
    ("4", "expert"),
    ("5", "master"),
    ("6", "remaster"),
];

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// Parse `&key=value` blocks. A value runs from its `&key=` line until the
/// next `&key=` line or EOF (inote bodies span many lines with no `&` prefix
/// of their own).
fn parse_maidata(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut buf: Vec<&str> = Vec::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('&') {
            if let Some((key, first_value_line)) = rest.split_once('=') {
                if let Some(prev_key) = current_key.take() {
                    out.insert(prev_key, buf.join("\n"));
                }
                current_key = Some(key.to_string());
                buf = vec![first_value_line];
                continue;
            }
        }
        if current_key.is_some() {
            buf.push(line);
        }
    }
    if let Some(prev_key) = current_key {
        out.insert(prev_key, buf.join("\n"));
    }
    out
}

struct SheetRef {
    id: i64,
    sheet_expr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let songs_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../songs"));

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // song title (as stored) -> song id, loaded once and matched by NFC form.
    let songs: Vec<(i64, String)> = sqlx::query_as("SELECT id, title FROM songs WHERE title IS NOT NULL")
        .fetch_all(&pool)
        .await?;
    let mut song_by_title: HashMap<String, i64> = HashMap::new();
    for (id, title) in songs {
        song_by_title.insert(nfc(&title), id);
    }

    let mut seeded = 0usize;
    let mut warnings = 0usize;
    let mut skipped_songs = 0usize;

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&songs_dir)
        .map_err(|e| format!("reading songs dir '{}': {e}", songs_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    for song_dir in entries {
        let maidata_path = song_dir.join("maidata.txt");
        let dir_name = song_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let Ok(text) = std::fs::read_to_string(&maidata_path) else {
            continue; // no maidata.txt in this directory — not a song folder
        };
        let kv = parse_maidata(&text);
        let Some(title) = kv.get("title").map(|s| s.trim()) else {
            eprintln!("warning: {dir_name}: no &title= in maidata.txt, skipping");
            skipped_songs += 1;
            continue;
        };

        let Some(&song_id) = song_by_title.get(&nfc(title)) else {
            eprintln!("warning: {dir_name}: no catalog song matches title '{title}', skipping");
            skipped_songs += 1;
            continue;
        };

        for (slot, difficulty) in SLOT_DIFFICULTIES {
            let Some(inote) = kv.get(&format!("inote_{slot}")) else { continue };
            if inote.trim().is_empty() {
                continue;
            }

            let sheet: Option<SheetRef> = sqlx::query_as!(
                SheetRef,
                "SELECT id, sheet_expr FROM sheets WHERE song_id_fk = $1 AND difficulty = $2",
                song_id,
                difficulty
            )
            .fetch_optional(&pool)
            .await?;

            let Some(sheet) = sheet else {
                if *difficulty != "master" && *difficulty != "remaster" {
                    eprintln!(
                        "warning: {dir_name}: maidata has '{difficulty}' but no matching sheet in the catalog"
                    );
                    warnings += 1;
                }
                continue;
            };

            upsert_chart(&pool, sheet.id, &sheet.sheet_expr, inote).await?;
            seeded += 1;
        }
    }

    println!("seeded {seeded} sheets, {warnings} warnings, {skipped_songs} songs skipped entirely");
    Ok(())
}

async fn upsert_chart(pool: &PgPool, sheet_id: i64, sheet_expr: &str, content: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, approved_at) \
         VALUES ($1, $2, 'maimai-simai', $3, now()) \
         ON CONFLICT (sheet_id, format) DO UPDATE \
           SET content = EXCLUDED.content, \
               version = charts.version + 1, \
               updated_at = now()",
    )
    .bind(sheet_id)
    .bind(sheet_expr)
    .bind(content)
    .execute(pool)
    .await?;
    Ok(())
}
