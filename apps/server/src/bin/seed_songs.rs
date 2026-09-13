//! Bulk-seed charts from local maidata.txt files (one per song directory under
//! `songs/`, e.g. `songs/larva/maidata.txt`) into the DB, keyed by sheet_expr.
//! Same upsert target as seed_chart, batched across every difficulty a
//! maidata file provides.
//!
//!   cargo run --bin seed_songs [-- <path/to/songs-dir>] [--allow-unmatched]
//!
//! Matches each maidata's &title= against the catalog's songs.title (NFC-
//! normalized, since upstream data can use a different but visually
//! identical Unicode form — e.g. U+212B ANGSTROM SIGN vs U+00C5 Å). A
//! difficulty present in the maidata with no matching sheet in the catalog
//! is reported (song legitimately doesn't have that difficulty, or the
//! catalog is stale) — except master/remaster, which are commonly and
//! expectedly absent for many songs, so those are skipped silently.
//!
//! Anything that fails to match — a title matching no catalog song at all
//! (upstream renamed it, most likely) or a non-master difficulty with no
//! matching sheet — exits non-zero unless --allow-unmatched is passed. This
//! runs unattended in CI (prod-data-and-infra issue 04), where silent skips
//! mean nobody finds out a chart quietly stopped loading.
//!
//! Re-running is safe and cheap: identical chart text is a no-op that writes
//! nothing and does not bump the revision, and is reported separately from
//! sheets actually seeded.

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use server::chart_revision::{ChartToApply, apply_chart_revisions};
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

/// Anything that failed to match is a chart that has quietly stopped
/// loading — a renamed title and a missing difficulty are the same failure
/// seen at two granularities, so both fail the run. `--allow-unmatched`
/// is the single escape hatch for a knowingly-stale catalog.
fn exit_code(unmatched_titles: usize, unmatched_difficulties: usize, allow_unmatched: bool) -> i32 {
    if allow_unmatched || unmatched_titles + unmatched_difficulties == 0 {
        0
    } else {
        1
    }
}

/// Parse `&key=value` blocks. A value runs from its `&key=` line until the
/// next `&key=` line or EOF (inote bodies span many lines with no `&` prefix
/// of their own).
fn parse_maidata(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut buf: Vec<&str> = Vec::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix('&')
            && let Some((key, first_value_line)) = rest.split_once('=')
        {
            if let Some(prev_key) = current_key.take() {
                out.insert(prev_key, buf.join("\n"));
            }
            current_key = Some(key.to_string());
            buf = vec![first_value_line];
            continue;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let mut allow_unmatched = false;
    let mut songs_dir_arg: Option<PathBuf> = None;
    for arg in std::env::args().skip(1) {
        if arg == "--allow-unmatched" {
            allow_unmatched = true;
        } else {
            songs_dir_arg = Some(PathBuf::from(arg));
        }
    }
    let songs_dir =
        songs_dir_arg.unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../songs"));

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // song title (as stored) -> song id, loaded once and matched by NFC form.
    //
    // ORDER BY id because 78 catalog titles are held by more than one song
    // (1845 songs, 1767 distinct titles) and the last insert wins. Without it,
    // Postgres is free to return rows in any order, so which song a duplicated
    // title resolves to varied between runs — two seeds of identical input into
    // identically-bootstrapped databases produced 6571 and 6568 charts. This makes
    // the choice reproducible; it does not make it *right*. Resolving a duplicated
    // title from `&title=` alone is not possible, and treating it as unmatched
    // rather than guessing is tracked in prod-data-and-infra issue 03.
    let songs: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, title FROM songs WHERE title IS NOT NULL ORDER BY id")
            .fetch_all(&pool)
            .await?;
    let mut song_by_title: HashMap<String, i64> = HashMap::new();
    for (id, title) in songs {
        song_by_title.insert(nfc(&title), id);
    }

    // Every sheet in the catalog, keyed the way the loop below looks them up.
    // This replaces one `SELECT ... WHERE song_id_fk = $1 AND difficulty = $2` per
    // song-difficulty pair — ~9.6k round trips for the full collection — with one.
    let sheet_rows: Vec<(i64, String, i64, String)> =
        sqlx::query_as("SELECT song_id_fk, difficulty, id, sheet_expr FROM sheets")
            .fetch_all(&pool)
            .await?;
    let sheet_by_song_difficulty: HashMap<(i64, String), (i64, String)> = sheet_rows
        .into_iter()
        .map(|(song_id, difficulty, id, expr)| ((song_id, difficulty), (id, expr)))
        .collect();

    // `applied` counts real writes only. Counting attempts instead makes a
    // re-seed of unchanged data report the same number as the first run,
    // which is precisely the log line CI reads to tell whether anything
    // happened.
    let mut applied = 0usize;
    let mut unchanged = 0usize;
    // Every chart to write, collected across all songs and applied in one batch.
    let mut pending: Vec<ChartToApply> = Vec::new();
    let mut skipped_songs = 0usize;
    let mut unmatched_titles: Vec<String> = Vec::new();
    let mut unmatched_difficulties: Vec<String> = Vec::new();

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&songs_dir)
        .map_err(|e| format!("reading songs dir '{}': {e}", songs_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    for song_dir in entries {
        let maidata_path = song_dir.join("maidata.txt");
        let dir_name = song_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
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
            unmatched_titles.push(format!("{dir_name}: '{title}'"));
            skipped_songs += 1;
            continue;
        };

        for (slot, difficulty) in SLOT_DIFFICULTIES {
            let Some(inote) = kv.get(&format!("inote_{slot}")) else {
                continue;
            };
            if inote.trim().is_empty() {
                continue;
            }

            let Some((sheet_id, sheet_expr)) =
                sheet_by_song_difficulty.get(&(song_id, (*difficulty).to_string()))
            else {
                if *difficulty != "master" && *difficulty != "remaster" {
                    eprintln!(
                        "warning: {dir_name}: maidata has '{difficulty}' but no matching sheet in the catalog"
                    );
                    unmatched_difficulties.push(format!("{dir_name}: '{difficulty}'"));
                }
                continue;
            };

            pending.push(ChartToApply {
                sheet_id: *sheet_id,
                sheet_expr: sheet_expr.clone(),
                content: inote.clone(),
            });
        }
    }

    // One transaction for every chart, rather than one per chart. The whole batch
    // is built first so nothing is held open while walking 1927 directories off
    // disk, and so `catalog_meta.revision` advances once for the run instead of
    // once per chart — see `apply_chart_revisions`.
    if !pending.is_empty() {
        let mut tx = pool.begin().await?;
        let outcome = apply_chart_revisions(&mut tx, "maimai-simai", &pending).await?;
        tx.commit().await?;
        applied = outcome.applied;
        unchanged = outcome.unchanged;
    }

    println!(
        "seeded {applied} sheets ({unchanged} already up to date), \
         {} unmatched difficulties, {} unmatched titles, {skipped_songs} songs skipped entirely",
        unmatched_difficulties.len(),
        unmatched_titles.len()
    );

    report("unmatched titles", &unmatched_titles);
    report("unmatched difficulties", &unmatched_difficulties);

    let code = exit_code(
        unmatched_titles.len(),
        unmatched_difficulties.len(),
        allow_unmatched,
    );
    if code != 0 {
        eprintln!("\nre-run with --allow-unmatched to accept these and exit 0");
        std::process::exit(code);
    }

    Ok(())
}

fn report(label: &str, entries: &[String]) {
    if entries.is_empty() {
        return;
    }
    eprintln!("\n{label} ({}):", entries.len());
    for entry in entries {
        eprintln!("  - {entry}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_run_exits_zero() {
        assert_eq!(exit_code(0, 0, false), 0);
    }

    #[test]
    fn an_unmatched_title_fails_the_run() {
        assert_eq!(exit_code(1, 0, false), 1);
    }

    // The regression ticket 09 exists for: a difficulty present in the
    // maidata with no catalog sheet is a chart that quietly stopped
    // loading, exactly like a renamed title. Warning and exiting 0 means
    // an unattended CI run stays green while charts go missing.
    #[test]
    fn an_unmatched_difficulty_fails_the_run_too() {
        assert_eq!(exit_code(0, 1, false), 1);
    }

    #[test]
    fn allow_unmatched_downgrades_both_to_warnings() {
        assert_eq!(exit_code(3, 2, true), 0);
    }
}
