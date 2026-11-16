//! Bulk-seed charts from local maidata.txt files (one per song directory under
//! `songs/`, e.g. `songs/larva/maidata.txt`) into the DB, keyed by sheet_expr.
//! Same upsert target as seed_chart, batched across every difficulty a
//! maidata file provides.
//!
//!   cargo run --bin seed_songs [-- <path/to/songs-dir>] [--allow-unmatched]
//!
//! Each folder is resolved to one catalog **song and chart type** (`resolve`):
//! a maidata file holds the charts of one type, and 81 catalog songs have
//! both a standard and a DX set with the same difficulty names, so a title
//! alone names two sheets per difficulty. The title (`&title=`, then the
//! folder name, since sample folders truncate titles at `&`) is matched by
//! NFC form first and by a looser fold second (NFKC, case, whitespace — the
//! upstream data uses full-width `～（！＆` where the files use ASCII). A
//! ` [DX]`/` [ST]` suffix names the type outright; otherwise the maidata's
//! `&lv_N=` levels are scored against each candidate type's sheet levels,
//! and only a unique best match seeds. A tie or a level set matching nothing
//! is *ambiguous* and reported rather than guessed — the alternative, the
//! last-inserted sheet winning, put standard charts on DX sheets.
//!
//! A difficulty present in the maidata with no matching sheet in the catalog
//! is reported (song legitimately doesn't have that difficulty, or the
//! catalog is stale) — except master/remaster, which are commonly and
//! expectedly absent for many songs, so those are skipped silently. UTAGE
//! folders (`&lv_7=`, charts in slot 7) are counted and skipped: the catalog
//! keys those sheets by a kanji tag, not by slot — `catalog-difficulties`
//! issue 03 — so they are not a failure of this run.
//!
//! Anything that fails to match — a title matching no catalog song at all
//! (upstream renamed it, most likely), an ambiguous type, or a non-master
//! difficulty with no matching sheet — exits non-zero unless
//! --allow-unmatched is passed. This runs unattended in CI
//! (prod-data-and-infra issue 04), where silent skips mean nobody finds out
//! a chart quietly stopped loading.
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

/// The looser second-pass key: NFKC folds full-width punctuation onto ASCII
/// (`～` → `~`, `（` → `(`, `＆` → `&`), case is folded, and runs of
/// whitespace collapse to one space. Only consulted when the NFC form
/// matched nothing, so it cannot steal an exact match.
fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for c in s.nfkc().flat_map(char::to_lowercase) {
        if c.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space && !out.is_empty() {
            out.push(' ');
        }
        pending_space = false;
        out.push(c);
    }
    out
}

/// `"39 [DX]"` → (`"39"`, `Some("dx")`); `"39"` → (`"39"`, `None`).
fn split_type_suffix(name: &str) -> (&str, Option<&'static str>) {
    let name = name.trim();
    for (suffix, chart_type) in [("[DX]", "dx"), ("[ST]", "std")] {
        if let Some(base) = name.strip_suffix(suffix) {
            return (base.trim_end(), Some(chart_type));
        }
    }
    (name, None)
}

/// One catalog sheet, as the resolver and the chart loop need it.
#[derive(Debug, Clone, PartialEq)]
struct SheetRef {
    id: i64,
    sheet_expr: String,
    level: String,
}

/// The catalog, indexed for resolution. Built once from two queries.
#[derive(Debug, Default)]
struct Catalog {
    /// NFC title → every song holding it (78 titles are held by more than one).
    songs_by_title: HashMap<String, Vec<i64>>,
    /// `fold`ed title → the same, for the second pass.
    songs_by_fold: HashMap<String, Vec<i64>>,
    /// (song, type) → difficulty → sheet. UTAGE sheets are not loaded.
    sheets: HashMap<(i64, String), HashMap<String, SheetRef>>,
}

impl Catalog {
    fn add_song(&mut self, id: i64, title: &str) {
        self.songs_by_title.entry(nfc(title)).or_default().push(id);
        self.songs_by_fold.entry(fold(title)).or_default().push(id);
    }

    fn add_sheet(&mut self, song_id: i64, chart_type: &str, difficulty: &str, sheet: SheetRef) {
        self.sheets
            .entry((song_id, chart_type.to_string()))
            .or_default()
            .insert(difficulty.to_string(), sheet);
    }

    /// The chart types a song has sheets for, in a stable order.
    fn types_of(&self, song_id: i64) -> Vec<&str> {
        let mut types: Vec<&str> = self
            .sheets
            .keys()
            .filter(|(id, _)| *id == song_id)
            .map(|(_, t)| t.as_str())
            .collect();
        types.sort_unstable();
        types
    }
}

/// What one folder claims about itself.
#[derive(Debug)]
struct Claim<'a> {
    /// `&title=`, as written.
    title: &'a str,
    /// The folder name — the title's second chance, since the sample's
    /// `&title=` lines are cut at `&` while folder names are not.
    dir_name: &'a str,
    /// difficulty → `&lv_N=` as written, for the difficulties present.
    levels: Vec<(&'static str, String)>,
}

#[derive(Debug, PartialEq)]
enum Resolution {
    Matched {
        song_id: i64,
        chart_type: String,
        /// Which key matched, for the log: `title`, `dir`, `title~`, `dir~`.
        via: &'static str,
    },
    /// No catalog song under any form of the title.
    NoSong,
    /// Several (song, type) candidates and the levels do not single one out.
    Ambiguous(Vec<(i64, String)>),
}

fn resolve<'a>(claim: &Claim, catalog: &'a Catalog) -> Resolution {
    let (title_base, title_type) = split_type_suffix(claim.title);
    let (dir_base, dir_type) = split_type_suffix(claim.dir_name);
    let wanted_type = title_type.or(dir_type);

    // An empty key never matches: a folder with a blank `&title=` must not
    // land on the one catalog song whose title folds to nothing.
    let lookup = |index: &'a HashMap<String, Vec<i64>>, key: String| {
        (!key.is_empty()).then(|| index.get(&key)).flatten()
    };
    let attempts: [(&'static str, Option<&Vec<i64>>); 4] = [
        ("title", lookup(&catalog.songs_by_title, nfc(title_base))),
        ("dir", lookup(&catalog.songs_by_title, nfc(dir_base))),
        ("title~", lookup(&catalog.songs_by_fold, fold(title_base))),
        ("dir~", lookup(&catalog.songs_by_fold, fold(dir_base))),
    ];
    let Some((via, songs)) = attempts
        .into_iter()
        .find_map(|(via, songs)| songs.map(|s| (via, s)))
    else {
        return Resolution::NoSong;
    };

    let candidates: Vec<(i64, String)> = songs
        .iter()
        .flat_map(|&song_id| {
            catalog
                .types_of(song_id)
                .into_iter()
                .filter(|t| wanted_type.is_none_or(|w| w == *t))
                .map(move |t| (song_id, t.to_string()))
        })
        .collect();

    match candidates.as_slice() {
        [] => Resolution::NoSong,
        [(song_id, chart_type)] => Resolution::Matched {
            song_id: *song_id,
            chart_type: chart_type.clone(),
            via,
        },
        _ => {
            // Score each candidate by how many of the folder's levels agree
            // with that type's sheet levels; a unique best wins.
            let scored: Vec<(usize, &(i64, String))> = candidates
                .iter()
                .map(|c| (level_agreement(claim, catalog.sheets.get(c)), c))
                .collect();
            let best = scored.iter().map(|(s, _)| *s).max().unwrap_or(0);
            let winners: Vec<&(i64, String)> = scored
                .iter()
                .filter(|(s, _)| *s == best)
                .map(|(_, c)| *c)
                .collect();
            match (best, winners.as_slice()) {
                (0, _) | (_, [_, _, ..]) => Resolution::Ambiguous(candidates),
                (_, [(song_id, chart_type)]) => Resolution::Matched {
                    song_id: *song_id,
                    chart_type: chart_type.clone(),
                    via,
                },
                (_, []) => Resolution::NoSong,
            }
        }
    }
}

/// How many of the claim's levels equal the sheet levels of one (song, type).
fn level_agreement(claim: &Claim, sheets: Option<&HashMap<String, SheetRef>>) -> usize {
    let Some(sheets) = sheets else { return 0 };
    claim
        .levels
        .iter()
        .filter(|(difficulty, level)| {
            sheets
                .get(*difficulty)
                .is_some_and(|s| fold(&s.level) == fold(level))
        })
        .count()
}

fn nfc(s: &str) -> String {
    s.nfc().collect()
}

/// Anything that failed to match is a chart that has quietly stopped
/// loading — a renamed title and a missing difficulty are the same failure
/// seen at two granularities, so both fail the run. `--allow-unmatched`
/// is the single escape hatch for a knowingly-stale catalog.
fn exit_code(unmatched_titles: usize, unmatched_difficulties: usize, allow_unmatched: bool) -> i32 {
    // Ambiguous folders are counted into `unmatched_titles` by the caller:
    // a folder that could not be pinned to one (song, type) seeded nothing,
    // which is the same outcome as a title that matched no song.
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

    // The whole catalog in two queries, indexed for `resolve`. A title held
    // by more than one song (78 of them) keeps every holder; the folder's
    // levels pick between them, or the folder is reported as ambiguous —
    // never the last-inserted row winning, which is what a title → id map
    // did before. ORDER BY keeps the candidate lists, and so the reports,
    // stable between runs.
    let mut catalog = Catalog::default();
    let songs: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, title FROM songs WHERE title IS NOT NULL ORDER BY id")
            .fetch_all(&pool)
            .await?;
    for (id, title) in &songs {
        catalog.add_song(*id, title);
    }
    // UTAGE sheets are keyed by a kanji tag, not a slot (catalog-difficulties
    // 03); loading them would only make every UTAGE song look ambiguous.
    let sheet_rows: Vec<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT song_id_fk, type, difficulty, id, sheet_expr, level \
         FROM sheets WHERE type <> 'utage' ORDER BY id",
    )
    .fetch_all(&pool)
    .await?;
    for (song_id, chart_type, difficulty, id, sheet_expr, level) in sheet_rows {
        catalog.add_sheet(
            song_id,
            &chart_type,
            &difficulty,
            SheetRef {
                id,
                sheet_expr,
                level,
            },
        );
    }

    // `applied` counts real writes only. Counting attempts instead makes a
    // re-seed of unchanged data report the same number as the first run,
    // which is precisely the log line CI reads to tell whether anything
    // happened.
    let mut applied = 0usize;
    let mut unchanged = 0usize;
    // Every chart to write, collected across all songs and applied in one batch.
    let mut pending: Vec<ChartToApply> = Vec::new();
    let mut skipped_songs = 0usize;
    let mut skipped_utage = 0usize;
    let mut unmatched_titles: Vec<String> = Vec::new();
    let mut unmatched_difficulties: Vec<String> = Vec::new();
    let mut loosely_matched: Vec<String> = Vec::new();

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

        // Slot 7 is UTAGE. Those sheets are not loaded (see above), so a
        // folder that is only UTAGE would otherwise be reported as unmatched
        // when it is merely not seedable yet.
        if kv.contains_key("lv_7") || kv.contains_key("inote_7") {
            skipped_utage += 1;
            continue;
        }

        let claim = Claim {
            title,
            dir_name: &dir_name,
            levels: SLOT_DIFFICULTIES
                .iter()
                .filter_map(|(slot, difficulty)| {
                    kv.get(&format!("lv_{slot}"))
                        .map(|lv| (*difficulty, lv.trim().to_string()))
                })
                .collect(),
        };
        let (song_id, chart_type) = match resolve(&claim, &catalog) {
            Resolution::Matched {
                song_id,
                chart_type,
                via,
            } => {
                if via != "title" {
                    loosely_matched.push(format!("{dir_name}: '{title}' via {via} → {chart_type}"));
                }
                (song_id, chart_type)
            }
            Resolution::NoSong => {
                eprintln!("warning: {dir_name}: no catalog song matches title '{title}', skipping");
                unmatched_titles.push(format!("{dir_name}: '{title}'"));
                skipped_songs += 1;
                continue;
            }
            Resolution::Ambiguous(candidates) => {
                eprintln!(
                    "warning: {dir_name}: '{title}' could be any of {candidates:?} and its levels decide nothing, skipping"
                );
                unmatched_titles.push(format!("{dir_name}: '{title}' (ambiguous: {candidates:?})"));
                skipped_songs += 1;
                continue;
            }
        };
        let sheets = catalog
            .sheets
            .get(&(song_id, chart_type))
            .expect("resolve only returns (song, type) pairs that have sheets");

        for (slot, difficulty) in SLOT_DIFFICULTIES {
            let Some(inote) = kv.get(&format!("inote_{slot}")) else {
                continue;
            };
            if inote.trim().is_empty() {
                continue;
            }

            let Some(SheetRef {
                id: sheet_id,
                sheet_expr,
                ..
            }) = sheets.get(*difficulty)
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
         {} unmatched difficulties, {} unmatched titles, {skipped_songs} songs skipped entirely, \
         {skipped_utage} UTAGE folders not seedable yet, {} matched loosely",
        unmatched_difficulties.len(),
        unmatched_titles.len(),
        loosely_matched.len()
    );

    report(
        "matched by folder name or loose fold (check these once)",
        &loosely_matched,
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

    // ── resolve ───────────────────────────────────────────────────────────

    fn sheet(expr: &str, level: &str) -> SheetRef {
        SheetRef {
            id: 0,
            sheet_expr: expr.to_string(),
            level: level.to_string(),
        }
    }

    /// A catalog with "39" in both types (different master levels), a
    /// DX-only "Halcyon", and two songs sharing the title "Dup".
    fn catalog() -> Catalog {
        let mut c = Catalog::default();
        c.add_song(1, "39");
        c.add_sheet(1, "std", "master", sheet("39|std|master", "12+"));
        c.add_sheet(1, "std", "expert", sheet("39|std|expert", "9"));
        c.add_sheet(1, "dx", "master", sheet("39|dx|master", "13"));
        c.add_sheet(1, "dx", "expert", sheet("39|dx|expert", "9"));
        c.add_song(2, "Halcyon");
        c.add_sheet(2, "dx", "master", sheet("Halcyon|dx|master", "13"));
        c.add_song(3, "Dup");
        c.add_sheet(3, "std", "master", sheet("Dup#3|std|master", "10"));
        c.add_song(4, "Dup");
        c.add_sheet(4, "std", "master", sheet("Dup#4|std|master", "11"));
        c.add_song(5, "Retribution ～ Cycle ～");
        c.add_sheet(5, "dx", "master", sheet("Retribution|dx|master", "14"));
        c.add_song(6, "\u{3000}");
        c.add_sheet(6, "dx", "master", sheet("blank|dx|master", "1"));
        c
    }

    fn claim<'a>(title: &'a str, dir: &'a str, levels: &[(&'static str, &str)]) -> Claim<'a> {
        Claim {
            title,
            dir_name: dir,
            levels: levels.iter().map(|(d, l)| (*d, l.to_string())).collect(),
        }
    }

    fn matched(song_id: i64, chart_type: &str, via: &'static str) -> Resolution {
        Resolution::Matched {
            song_id,
            chart_type: chart_type.to_string(),
            via,
        }
    }

    #[test]
    fn a_type_suffix_names_the_sheet_set() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("39 [DX]", "39 [DX]", &[]), &c),
            matched(1, "dx", "title")
        );
        assert_eq!(
            resolve(&claim("39 [ST]", "39", &[]), &c),
            matched(1, "std", "title")
        );
        assert_eq!(split_type_suffix("39 [DX] "), ("39", Some("dx")));
        assert_eq!(split_type_suffix("39"), ("39", None));
    }

    #[test]
    fn a_single_type_song_needs_no_hint() {
        assert_eq!(
            resolve(&claim("Halcyon", "Halcyon", &[]), &catalog()),
            matched(2, "dx", "title")
        );
    }

    // The bug this replaces: "39" has a standard and a DX master, and the
    // folder without a suffix used to land on whichever sheet the map kept.
    #[test]
    fn levels_pick_the_type_when_the_title_names_two() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("39", "39", &[("master", "13"), ("expert", "9")]), &c),
            matched(1, "dx", "title")
        );
        assert_eq!(
            resolve(&claim("39", "39", &[("master", "12+")]), &c),
            matched(1, "std", "title")
        );
    }

    #[test]
    fn a_tie_or_no_level_agreement_is_ambiguous_not_a_guess() {
        let c = catalog();
        // expert is 9 in both types: a tie.
        assert!(matches!(
            resolve(&claim("39", "39", &[("expert", "9")]), &c),
            Resolution::Ambiguous(ref v) if v.len() == 2
        ));
        // no levels at all, and levels matching neither type
        assert!(matches!(
            resolve(&claim("39", "39", &[]), &c),
            Resolution::Ambiguous(_)
        ));
        assert!(matches!(
            resolve(&claim("39", "39", &[("master", "15")]), &c),
            Resolution::Ambiguous(_)
        ));
    }

    #[test]
    fn a_duplicated_title_is_split_by_levels_too() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("Dup", "Dup", &[("master", "11")]), &c),
            matched(4, "std", "title")
        );
    }

    // Sample folders truncate `&title=` at `&`; the folder name does not.
    #[test]
    fn the_folder_name_is_the_titles_second_chance() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("Halc", "Halcyon", &[]), &c),
            matched(2, "dx", "dir")
        );
    }

    #[test]
    fn the_loose_fold_bridges_full_width_punctuation_and_case() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("Retribution ~ Cycle ~", "x", &[]), &c),
            matched(5, "dx", "title~")
        );
        assert_eq!(
            resolve(&claim("halcyon", "x", &[]), &c),
            matched(2, "dx", "title~")
        );
        assert_eq!(fold("Ａ  b\tC"), "a b c");
    }

    #[test]
    fn an_empty_title_matches_nothing() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("", "", &[("master", "1")]), &c),
            Resolution::NoSong
        );
        assert_eq!(resolve(&claim("", "\u{3000}", &[]), &c), Resolution::NoSong);
    }

    #[test]
    fn a_suffix_the_song_does_not_have_matches_nothing() {
        assert_eq!(
            resolve(&claim("Halcyon [ST]", "Halcyon [ST]", &[]), &catalog()),
            Resolution::NoSong
        );
    }
}
