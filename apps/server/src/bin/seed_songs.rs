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
//! expectedly absent for many songs, so those are skipped silently.
//!
//! UTAGE folders (`&inote_7=`) resolve separately (`resolve_utage`): the
//! catalog keys each UTAGE sheet by a tag (`【協】`), which `&lv_7=` names by
//! its `difficulties.code` (`協`), and titles them `[協]X` or plain `X` where
//! the folder says `[協] X`. Only a unique sheet seeds. One side of a buddy
//! chart (`[1P]`/`[2P]`) is counted and set aside: the catalog has one sheet
//! for both sides. An unmatched or ambiguous UTAGE folder fails the run like
//! any other unmatched chart.
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
use simai::maidata::{self, Maidata, MaidataError};
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use unicode_normalization::UnicodeNormalization;

/// The standard slots of a parsed file, each with the catalog difficulty
/// code it seeds (`maidata::standard_difficulty`), in slot order.
fn standard_slots(m: &Maidata) -> impl Iterator<Item = (&'static str, &maidata::Difficulty)> {
    m.difficulties
        .iter()
        .filter_map(|d| maidata::standard_difficulty(d.slot).map(|code| (code, d)))
}

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
    songs_by_title: HashMap<String, Vec<Uuid>>,
    /// `fold`ed title → the same, for the second pass.
    songs_by_fold: HashMap<String, Vec<Uuid>>,
    /// (song, type) → difficulty → sheet. UTAGE sheets are not loaded.
    sheets: HashMap<(Uuid, String), HashMap<String, SheetRef>>,
}

impl Catalog {
    fn add_song(&mut self, id: Uuid, title: &str) {
        self.songs_by_title.entry(nfc(title)).or_default().push(id);
        self.songs_by_fold.entry(fold(title)).or_default().push(id);
    }

    fn add_sheet(&mut self, song_id: Uuid, chart_type: &str, difficulty: &str, sheet: SheetRef) {
        self.sheets
            .entry((song_id, chart_type.to_string()))
            .or_default()
            .insert(difficulty.to_string(), sheet);
    }

    /// The chart types a song has sheets for, in a stable order.
    fn types_of(&self, song_id: Uuid) -> Vec<&str> {
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
        song_id: Uuid,
        chart_type: String,
        /// Which key matched, for the log: `title`, `dir`, `title~`, `dir~`.
        via: &'static str,
    },
    /// No catalog song under any form of the title.
    NoSong,
    /// Several (song, type) candidates and the levels do not single one out.
    Ambiguous(Vec<(Uuid, String)>),
}

fn resolve<'a>(claim: &Claim, catalog: &'a Catalog) -> Resolution {
    let (title_base, title_type) = split_type_suffix(claim.title);
    let (dir_base, dir_type) = split_type_suffix(claim.dir_name);
    let wanted_type = title_type.or(dir_type);

    // An empty key never matches: a folder with a blank `&title=` must not
    // land on the one catalog song whose title folds to nothing.
    let lookup = |index: &'a HashMap<String, Vec<Uuid>>, key: String| {
        (!key.is_empty()).then(|| index.get(&key)).flatten()
    };
    let attempts: [(&'static str, Option<&Vec<Uuid>>); 4] = [
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

    let candidates: Vec<(Uuid, String)> = songs
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
            let scored: Vec<(usize, &(Uuid, String))> = candidates
                .iter()
                .map(|c| (level_agreement(claim, catalog.sheets.get(c)), c))
                .collect();
            let best = scored.iter().map(|(s, _)| *s).max().unwrap_or(0);
            let winners: Vec<&(Uuid, String)> = scored
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

// ── UTAGE (maidata slot 7) ───────────────────────────────────────────────────

/// The catalog's UTAGE sheets, indexed by (NFC title, difficulty). Kept apart
/// from [`Catalog`]: a UTAGE song shares its plain title with the standard
/// song it parodies, and the two must never be candidates for each other.
#[derive(Debug, Default)]
struct UtageIndex {
    sheets: HashMap<(String, String), Vec<SheetRef>>,
    /// `difficulties.code` → `difficulties.difficulty`: `協` → `【協】`.
    /// maidata's `&lv_7=` is the code, so the lookup needs no transformation.
    codes: HashMap<String, String>,
}

impl UtageIndex {
    fn add_code(&mut self, code: &str, difficulty: &str) {
        self.codes.insert(code.to_string(), difficulty.to_string());
    }

    fn add_sheet(&mut self, title: &str, difficulty: &str, sheet: SheetRef) {
        self.sheets
            .entry((nfc(title), difficulty.to_string()))
            .or_default()
            .push(sheet);
    }
}

/// What one slot-7 folder claims about itself.
#[derive(Debug)]
struct UtageClaim<'a> {
    /// `&title=`, e.g. `[宴] 人マニア [14?]`.
    title: &'a str,
    /// e.g. `[X] 人マニア`.
    dir_name: &'a str,
    /// `&lv_7=`, the bare tag: `X`.
    tag: &'a str,
}

/// A slot-7 folder's claim and its chart text, or `None` when the folder has
/// no slot-7 chart to seed.
fn utage_claim<'a>(m: &'a Maidata, dir_name: &'a str) -> Option<(UtageClaim<'a>, &'a String)> {
    let slot = utage_slot(m)?;
    let inote = slot.inote.as_ref()?;
    let claim = UtageClaim {
        title: &m.title,
        dir_name,
        tag: slot.lv.as_deref().unwrap_or(""),
    };
    Some((claim, inote))
}

/// Slot 7, when the file has a UTAGE level or chart there. A bare `&des_7=`
/// does not make a folder UTAGE.
fn utage_slot(m: &Maidata) -> Option<&maidata::Difficulty> {
    m.difficulties
        .iter()
        .find(|d| d.slot == 7 && (d.lv.is_some() || d.inote.is_some()))
}

#[derive(Debug, PartialEq)]
enum UtageResolution {
    Matched(SheetRef),
    /// One side of a two-player chart. The catalog holds one sheet for both
    /// sides and a sheet holds one chart, so neither side is seeded.
    Buddy,
    /// `&lv_7=` is no `difficulties.code`.
    UnknownTag,
    NoSheet,
    /// Several sheets fit and nothing in the folder says which.
    Ambiguous(Vec<String>),
}

/// Every title form the folder offers is looked up under its tag's difficulty,
/// and the candidates are pooled: only a single sheet across all forms seeds.
/// Taking the first form that hits instead would pick silently when two
/// catalog charts carry the same tag under different title forms, and a
/// mismatched chart renders as if it were correct. Matching is exact (NFC);
/// there is no loose fold here, by design.
fn resolve_utage(claim: &UtageClaim, index: &UtageIndex) -> UtageResolution {
    let dir = claim.dir_name.trim();
    let title = claim.title.trim();
    let tag = claim.tag.trim();

    // The folder suffix is the reliable signal; one sample spells the title
    // prefix バティ rather than バディ.
    let buddy = ["[1P]", "[2P]"].iter().any(|side| dir.ends_with(side))
        || ["/1P]", "/2P]"].iter().any(|side| title.contains(side));
    if buddy {
        return UtageResolution::Buddy;
    }

    let Some(difficulty) = index.codes.get(tag) else {
        return UtageResolution::UnknownTag;
    };

    let plain = strip_level(strip_tag(title));
    let forms = [
        join_tag(dir),              // `[X] 人マニア` → `[X]人マニア`
        strip_tag(dir).to_string(), // `[光] Reach For The Stars` → the plain title
        format!("[{tag}]{plain}"),  // `[宴] WE'RE BACK!! [12+?]` → `[逆]WE'RE BACK!!`
        plain.to_string(),
    ];

    let mut found: Vec<&SheetRef> = Vec::new();
    for form in forms.iter().filter(|f| !f.is_empty()) {
        let Some(sheets) = index.sheets.get(&(nfc(form), difficulty.clone())) else {
            continue;
        };
        for sheet in sheets {
            if !found.iter().any(|f| f.sheet_expr == sheet.sheet_expr) {
                found.push(sheet);
            }
        }
    }

    match found.as_slice() {
        [] => UtageResolution::NoSheet,
        [sheet] => UtageResolution::Matched((*sheet).clone()),
        many => UtageResolution::Ambiguous(many.iter().map(|s| s.sheet_expr.clone()).collect()),
    }
}

/// Charts whose sheet exactly one folder resolved to, and a report line for
/// every sheet several folders resolved to. None of a colliding sheet's
/// folders is seeded: nothing says which of them is the catalog's chart.
fn split_collisions(
    matches: HashMap<String, Vec<(String, ChartToApply)>>,
) -> (Vec<ChartToApply>, Vec<String>) {
    let mut unique = Vec::new();
    let mut colliding = Vec::new();
    let mut matches: Vec<_> = matches.into_iter().collect();
    matches.sort_by(|a, b| a.0.cmp(&b.0)); // stable report order
    for (sheet_expr, mut folders) in matches {
        if folders.len() == 1 {
            unique.push(folders.pop().unwrap().1);
        } else {
            let names: Vec<String> = folders.into_iter().map(|(dir, _)| dir).collect();
            colliding.push(format!("{names:?} all resolve to {sheet_expr}"));
        }
    }
    (unique, colliding)
}

/// `[X] 人マニア` → (`[X]`, `人マニア`); `None` unless it opens with a bracket.
fn split_tag(s: &str) -> Option<(&str, &str)> {
    let end = s.strip_prefix('[').and(s.find(']'))?;
    Some((&s[..=end], s[end + 1..].trim_start()))
}

/// `[X] 人マニア` → `人マニア`. Anything not opening with a bracket is unchanged.
fn strip_tag(s: &str) -> &str {
    split_tag(s).map_or(s, |(_, rest)| rest)
}

/// `[X] 人マニア` → `[X]人マニア`: the catalog writes no space after the tag.
fn join_tag(s: &str) -> String {
    split_tag(s).map_or_else(|| s.to_string(), |(tag, rest)| format!("{tag}{rest}"))
}

/// `人マニア [14?]` → `人マニア`. Only a trailing bracket that reads as a level
/// (`14?`, `13+?`, `*`) is cut, so a title ending in its own bracket survives.
fn strip_level(s: &str) -> &str {
    let s = s.trim_end();
    if let Some(body) = s.strip_suffix(']')
        && let Some(open) = body.rfind('[')
    {
        let level = &body[open + 1..];
        if !level.is_empty()
            && level
                .chars()
                .all(|c| c.is_ascii_digit() || "+?*".contains(c))
        {
            return body[..open].trim_end();
        }
    }
    s
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
    let songs: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, title FROM songs WHERE title IS NOT NULL ORDER BY source_index, song_id",
    )
    .fetch_all(&pool)
    .await?;
    for (id, title) in &songs {
        catalog.add_song(*id, title);
    }
    // UTAGE sheets go to their own index below: their plain titles collide
    // with the standard songs they parody.
    let sheet_rows: Vec<(Uuid, String, String, i64, String, String)> = sqlx::query_as(
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

    let mut utage = UtageIndex::default();
    let codes: Vec<(String, String)> = sqlx::query_as("SELECT code, difficulty FROM difficulties")
        .fetch_all(&pool)
        .await?;
    for (code, difficulty) in &codes {
        utage.add_code(code, difficulty);
    }
    let utage_rows: Vec<(String, String, i64, String, String)> = sqlx::query_as(
        "SELECT s.title, sh.difficulty, sh.id, sh.sheet_expr, sh.level \
         FROM sheets sh JOIN songs s ON s.id = sh.song_id_fk \
         WHERE sh.type = 'utage' AND s.title IS NOT NULL AND sh.difficulty IS NOT NULL \
         ORDER BY sh.id",
    )
    .fetch_all(&pool)
    .await?;
    for (title, difficulty, id, sheet_expr, level) in utage_rows {
        utage.add_sheet(
            &title,
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
    let mut buddy_utage: Vec<String> = Vec::new();
    // sheet_expr → (folder, chart). Held back from `pending` until every
    // folder is read: two folders landing on one sheet is a collision, and
    // the batch would otherwise keep whichever came last without a word.
    let mut utage_matches: HashMap<String, Vec<(String, ChartToApply)>> = HashMap::new();
    let mut unmatched_utage: Vec<String> = Vec::new();
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
        let m = match maidata::parse(&text) {
            Ok(m) => m,
            Err(MaidataError::MissingTitle) => {
                eprintln!("warning: {dir_name}: no &title= in maidata.txt, skipping");
                skipped_songs += 1;
                continue;
            }
        };
        let title = m.title.as_str();

        // Slot 7 is UTAGE. No sample folder mixes it with slots 2-6, so the
        // folder is done once its UTAGE chart is resolved.
        if utage_slot(&m).is_some() {
            let Some((claim, inote)) = utage_claim(&m, &dir_name) else {
                continue; // a level with no chart: nothing to seed
            };
            let tag = claim.tag;
            match resolve_utage(&claim, &utage) {
                UtageResolution::Matched(sheet) => utage_matches
                    .entry(sheet.sheet_expr.clone())
                    .or_default()
                    .push((
                        dir_name.clone(),
                        ChartToApply {
                            sheet_id: sheet.id,
                            sheet_expr: sheet.sheet_expr,
                            content: inote.clone(),
                        },
                    )),
                UtageResolution::Buddy => buddy_utage.push(dir_name.clone()),
                UtageResolution::UnknownTag => {
                    unmatched_utage.push(format!("{dir_name}: &lv_7={tag} is no difficulty code"))
                }
                UtageResolution::NoSheet => {
                    unmatched_utage.push(format!("{dir_name}: '{title}' tag {tag}: no UTAGE sheet"))
                }
                UtageResolution::Ambiguous(exprs) => {
                    unmatched_utage.push(format!("{dir_name}: '{title}' could be any of {exprs:?}"))
                }
            }
            continue;
        }

        let claim = Claim {
            title,
            dir_name: &dir_name,
            levels: standard_slots(&m)
                .filter_map(|(code, d)| d.lv.clone().map(|lv| (code, lv)))
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

        for (difficulty, slot) in standard_slots(&m) {
            let Some(inote) = &slot.inote else {
                continue; // a level without a chart, or a blank one
            };

            let Some(SheetRef {
                id: sheet_id,
                sheet_expr,
                ..
            }) = sheets.get(difficulty)
            else {
                if difficulty != "master" && difficulty != "remaster" {
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

    let (unique, colliding) = split_collisions(utage_matches);
    pending.extend(unique);
    unmatched_utage.extend(colliding);

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
         {} unmatched UTAGE, {} buddy UTAGE folders set aside, {} matched loosely",
        unmatched_difficulties.len(),
        unmatched_titles.len(),
        unmatched_utage.len(),
        buddy_utage.len(),
        loosely_matched.len()
    );

    report(
        "matched by folder name or loose fold (check these once)",
        &loosely_matched,
    );
    report("unmatched titles", &unmatched_titles);
    report("unmatched difficulties", &unmatched_difficulties);
    report("unmatched UTAGE", &unmatched_utage);
    report(
        "buddy UTAGE folders set aside (one catalog sheet for both sides)",
        &buddy_utage,
    );

    // An unmatched UTAGE folder is an unmatched title seen through slot 7.
    let code = exit_code(
        unmatched_titles.len() + unmatched_utage.len(),
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

    /// Song n of the test catalog; the resolver treats the key as opaque.
    fn song(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

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
        c.add_song(song(1), "39");
        c.add_sheet(song(1), "std", "master", sheet("39|std|master", "12+"));
        c.add_sheet(song(1), "std", "expert", sheet("39|std|expert", "9"));
        c.add_sheet(song(1), "dx", "master", sheet("39|dx|master", "13"));
        c.add_sheet(song(1), "dx", "expert", sheet("39|dx|expert", "9"));
        c.add_song(song(2), "Halcyon");
        c.add_sheet(song(2), "dx", "master", sheet("Halcyon|dx|master", "13"));
        c.add_song(song(3), "Dup");
        c.add_sheet(song(3), "std", "master", sheet("Dup#3|std|master", "10"));
        c.add_song(song(4), "Dup");
        c.add_sheet(song(4), "std", "master", sheet("Dup#4|std|master", "11"));
        c.add_song(song(5), "Retribution ～ Cycle ～");
        c.add_sheet(
            song(5),
            "dx",
            "master",
            sheet("Retribution|dx|master", "14"),
        );
        c.add_song(song(6), "\u{3000}");
        c.add_sheet(song(6), "dx", "master", sheet("blank|dx|master", "1"));
        c
    }

    fn claim<'a>(title: &'a str, dir: &'a str, levels: &[(&'static str, &str)]) -> Claim<'a> {
        Claim {
            title,
            dir_name: dir,
            levels: levels.iter().map(|(d, l)| (*d, l.to_string())).collect(),
        }
    }

    fn matched(song_id: Uuid, chart_type: &str, via: &'static str) -> Resolution {
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
            matched(song(1), "dx", "title")
        );
        assert_eq!(
            resolve(&claim("39 [ST]", "39", &[]), &c),
            matched(song(1), "std", "title")
        );
        assert_eq!(split_type_suffix("39 [DX] "), ("39", Some("dx")));
        assert_eq!(split_type_suffix("39"), ("39", None));
    }

    #[test]
    fn a_single_type_song_needs_no_hint() {
        assert_eq!(
            resolve(&claim("Halcyon", "Halcyon", &[]), &catalog()),
            matched(song(2), "dx", "title")
        );
    }

    // The bug this replaces: "39" has a standard and a DX master, and the
    // folder without a suffix used to land on whichever sheet the map kept.
    #[test]
    fn levels_pick_the_type_when_the_title_names_two() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("39", "39", &[("master", "13"), ("expert", "9")]), &c),
            matched(song(1), "dx", "title")
        );
        assert_eq!(
            resolve(&claim("39", "39", &[("master", "12+")]), &c),
            matched(song(1), "std", "title")
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
            matched(song(4), "std", "title")
        );
    }

    // Sample folders truncate `&title=` at `&`; the folder name does not.
    #[test]
    fn the_folder_name_is_the_titles_second_chance() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("Halc", "Halcyon", &[]), &c),
            matched(song(2), "dx", "dir")
        );
    }

    #[test]
    fn the_loose_fold_bridges_full_width_punctuation_and_case() {
        let c = catalog();
        assert_eq!(
            resolve(&claim("Retribution ~ Cycle ~", "x", &[]), &c),
            matched(song(5), "dx", "title~")
        );
        assert_eq!(
            resolve(&claim("halcyon", "x", &[]), &c),
            matched(song(2), "dx", "title~")
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

    // ── resolve_utage ─────────────────────────────────────────────────────
    // Every claim below is the header of a real sample folder, and every
    // catalog row a real catalog row.

    fn utage_index() -> UtageIndex {
        let mut u = UtageIndex::default();
        for tag in ["X", "光", "蔵", "協", "逆", "充"] {
            u.add_code(tag, &format!("【{tag}】"));
        }
        u.add_sheet(
            "[X]人マニア",
            "【X】",
            sheet("[X]人マニア|utage|【X】", "14?"),
        );
        u.add_sheet(
            "Reach For The Stars",
            "【蔵】",
            sheet("(宴) Reach For The Stars|utage|【蔵】", "*"),
        );
        u.add_sheet(
            "Reach For The Stars",
            "【光】",
            sheet("(宴) Reach For The Stars (2)|utage|【光】", "*"),
        );
        u.add_sheet(
            "Love You",
            "【協】",
            sheet("(宴) Love You|utage|【協】", "*"),
        );
        u.add_sheet(
            "[協]Love You",
            "【協】",
            sheet("[協]Love You|utage|【協】", "12?"),
        );
        u.add_sheet(
            "[逆]WE'RE BACK!!",
            "【逆】",
            sheet("[逆]WE'RE BACK!!|utage|【逆】", "12+?"),
        );
        u.add_sheet(
            "[充]エンジェル ドリーム",
            "【充】",
            sheet("[充]エンジェル ドリーム|utage|【充】", "11?"),
        );
        u
    }

    fn utage<'a>(title: &'a str, dir: &'a str, tag: &'a str) -> UtageClaim<'a> {
        UtageClaim {
            title,
            dir_name: dir,
            tag,
        }
    }

    fn utage_matched(expr: &str) -> UtageResolution {
        UtageResolution::Matched(
            utage_index()
                .sheets
                .values()
                .flatten()
                .find(|s| s.sheet_expr == expr)
                .cloned()
                .unwrap(),
        )
    }

    /// The real file's header up to its first chart line, CRLF and all.
    const MAIDATA_X: &str = "&title=[宴] 人マニア [14?]\r\n&artist=原口沙輔 feat.重音テト\r\n&first=0\r\n&des=\r\n&converter_7=miriiaa×HotIceCream\r\n&lv_7=X\r\n&inote_7=(150){1},,,\n{8}\n";
    const MAIDATA_REACH: &str = "&title=[宴] Reach For The Stars\r\n&wholebpm=203\r\n&lv_7=光\r\n&inote_7=(203.0){1},\r\n{8}3b,2b,1b,8h[2:1],,7b,6b,5h[2:1],\r\n";
    const MAIDATA_WE_RE: &str = "&title=[宴] WE'RE BACK!! [12+?]\r\n&artist=Zekk\r\n&first=-0.05\r\n&des=\r\n&demo_seek=42.77\n&lv_7=逆\r\n&inote_7=(185)\n{1},\n";

    /// Parse a real header and resolve it the way the run does.
    fn resolve_file(maidata: &str, dir: &str) -> UtageResolution {
        let m = maidata::parse(maidata).unwrap();
        let (claim, _) = utage_claim(&m, dir).expect("a slot-7 chart");
        resolve_utage(&claim, &utage_index())
    }

    // All three mismatches at once. Slot: the chart is `&inote_7=`. Label:
    // `&lv_7=X` against the catalog's `【X】`. Title: the folder writes
    // `[X] 人マニア`, the catalog `[X]人マニア`.
    #[test]
    fn a_folder_name_without_the_space_after_its_tag_is_the_catalog_title() {
        assert_eq!(
            resolve_file(MAIDATA_X, "[X] 人マニア"),
            utage_matched("[X]人マニア|utage|【X】")
        );
    }

    // Older UTAGE songs carry the plain title; the tag tells two of them
    // apart, and the standard "Reach For The Stars" is never a candidate.
    #[test]
    fn the_tag_picks_between_utage_charts_of_one_plain_title() {
        assert_eq!(
            resolve_file(MAIDATA_REACH, "[光] Reach For The Stars"),
            utage_matched("(宴) Reach For The Stars (2)|utage|【光】")
        );
        assert_eq!(
            resolve_utage(
                &utage("[宴] Reach For The Stars", "[蔵] Reach For The Stars", "蔵"),
                &utage_index()
            ),
            utage_matched("(宴) Reach For The Stars|utage|【蔵】")
        );
    }

    // The folder name lost the apostrophe (`WE_RE`) and its tag differs
    // from `&lv_7=`; the `&title=` minus its `[宴]` and level is the match.
    #[test]
    fn the_maidata_title_without_its_prefix_and_level_also_matches() {
        assert_eq!(
            resolve_file(MAIDATA_WE_RE, "[戻] WE_RE BACK!!"),
            utage_matched("[逆]WE'RE BACK!!|utage|【逆】")
        );
    }

    #[test]
    fn a_folder_without_a_slot_7_chart_makes_no_claim() {
        let m = maidata::parse("&title=[宴] X\n&lv_7=X\n&inote_7=\n").unwrap();
        assert!(utage_claim(&m, "[X] X").is_none());
    }

    // Two catalog charts carry 協 for "Love You", one under each title form,
    // and the folder names no level. Either pick could be the wrong chart.
    #[test]
    fn a_folder_fitting_two_sheets_is_ambiguous_not_a_guess() {
        assert!(matches!(
            resolve_utage(&utage("[宴] Love You", "[協] Love You", "協"), &utage_index()),
            UtageResolution::Ambiguous(ref v) if v.len() == 2
        ));
    }

    #[test]
    fn either_side_of_a_buddy_chart_is_set_aside() {
        let u = utage_index();
        assert_eq!(
            resolve_utage(
                &utage(
                    "[宴/バディ/1P]エンジェル ドリーム [11?]",
                    "[充] エンジェル ドリーム [1P]",
                    "充"
                ),
                &u
            ),
            UtageResolution::Buddy
        );
        // One sample folder spells it バティ; the folder suffix still says.
        assert_eq!(
            resolve_utage(
                &utage(
                    "[宴/バティ/1P]エンジェル ドリーム",
                    "[充] エンジェル ドリーム [1P]",
                    "充"
                ),
                &u
            ),
            UtageResolution::Buddy
        );
    }

    #[test]
    fn a_tag_that_is_no_difficulty_code_is_reported() {
        assert_eq!(
            resolve_utage(
                &utage("[宴] 人マニア [14?]", "[X] 人マニア", "Y"),
                &utage_index()
            ),
            UtageResolution::UnknownTag
        );
    }

    #[test]
    fn a_utage_title_the_catalog_lacks_matches_nothing() {
        assert_eq!(
            resolve_utage(&utage("[宴] Nothing", "[X] Nothing", "X"), &utage_index()),
            UtageResolution::NoSheet
        );
    }

    fn chart(expr: &str) -> ChartToApply {
        ChartToApply {
            sheet_id: 0,
            sheet_expr: expr.to_string(),
            content: String::new(),
        }
    }

    #[test]
    fn two_folders_on_one_utage_sheet_seed_neither() {
        let mut m: HashMap<String, Vec<(String, ChartToApply)>> = HashMap::new();
        m.entry("a".into())
            .or_default()
            .push(("A".into(), chart("a")));
        m.entry("b".into())
            .or_default()
            .push(("B1".into(), chart("b")));
        m.entry("b".into())
            .or_default()
            .push(("B2".into(), chart("b")));

        let (unique, colliding) = split_collisions(m);

        assert_eq!(
            unique
                .iter()
                .map(|c| c.sheet_expr.as_str())
                .collect::<Vec<_>>(),
            ["a"]
        );
        assert_eq!(colliding, [r#"["B1", "B2"] all resolve to b"#]);
    }
}
