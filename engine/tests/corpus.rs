//! Corpus test: every fixture chart parses without panicking, and its event
//! counts match a committed snapshot.
//!
//! This is the net that catches a parser change nobody thought to write a unit
//! test for. It is deliberately coarse — it asserts *shape*, not correctness.
//! A misparsed slide that still produces one note per token slips through here
//! and is caught by the inline tests in `systems/parser/` instead.
//!
//! Fixtures are hand-written simai under `tests/fixtures/`, chart text only.
//! See `tests/fixtures/README.md` for what belongs there and why.
//!
//! `tests/fixtures/local/` is gitignored and optional: drop real `maidata.txt`
//! bodies there and the no-panic sweep picks them up, without their text ever
//! entering the repo. They are deliberately left out of the snapshot, so the
//! committed expectations do not depend on files a fresh clone will not have.

use maiscope_viewer::chart::{ChartEvent, parse_chart};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Counts per event kind for one chart. `BTreeMap` so the snapshot line is
/// ordered deterministically.
type Counts = BTreeMap<&'static str, usize>;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The snapshot lives beside this file rather than inside `fixtures/`, so the
/// glob below cannot pick it up as a chart and snapshot its own contents.
fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus-snapshot.txt")
}

/// Gitignored charts that stay on one machine. Absent on a fresh clone and in
/// CI, so nothing committed may depend on them.
fn local_dir() -> PathBuf {
    fixtures_dir().join("local")
}

/// Every `*.txt` directly inside `dir`, sorted by name. An absent directory
/// yields nothing rather than failing — `local/` is optional by design.
fn charts_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.expect("unreadable dir entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    files
}

/// The committed fixtures — the only ones the snapshot covers.
fn fixture_files() -> Vec<PathBuf> {
    charts_in(&fixtures_dir())
}

fn count_events(events: &[ChartEvent]) -> Counts {
    let mut counts = Counts::new();
    for event in events {
        let key = match event {
            ChartEvent::BpmChange(_) => "bpm_change",
            ChartEvent::ResolutionChange(_) => "resolution_change",
            ChartEvent::AbsoluteLength(_) => "absolute_length",
            ChartEvent::Rest => "rest",
            ChartEvent::NoteGroup(notes) => {
                *counts.entry("note_group").or_default() += 1;
                *counts.entry("note").or_default() += notes.len();
                continue;
            }
        };
        *counts.entry(key).or_default() += 1;
    }
    counts
}

/// One line per chart: `name: key=n key=n ...`.
fn render_snapshot() -> String {
    let mut out = String::new();
    for path in fixture_files() {
        let name = path.file_stem().unwrap().to_string_lossy();
        let text = fs::read_to_string(&path).expect("fixture is not valid UTF-8");
        let events = parse_chart(&text).expect("fixture failed to parse");
        let counts = count_events(&events);

        let rendered: Vec<String> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
        out.push_str(&format!("{name}: {}\n", rendered.join(" ")));
    }
    out
}

/// Parsing must not panic, whatever the chart contains. Separate from the
/// snapshot test so a panic reports as "this chart panics", not as a diff.
///
/// Sweeps the committed fixtures *and* any gitignored charts in `local/`. Real
/// charts are where unknown syntax actually comes from, so they are worth
/// running even though nothing may depend on them being present.
#[test]
fn every_fixture_parses_without_panicking() {
    let committed = fixture_files();
    assert!(
        !committed.is_empty(),
        "no fixtures in {} — the corpus test is vacuously passing",
        fixtures_dir().display()
    );

    for path in committed.into_iter().chain(charts_in(&local_dir())) {
        let text = fs::read_to_string(&path).expect("chart is not valid UTF-8");
        parse_chart(&text).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
    }
}

/// Event counts match `tests/fixtures/snapshot.txt`.
///
/// To accept an intended change: `UPDATE_SNAPSHOT=1 cargo test -p maiscope-viewer`,
/// then **read the diff before committing it**. A snapshot you regenerate without
/// reading is a snapshot that asserts nothing.
#[test]
fn event_counts_match_snapshot() {
    let snapshot_path = snapshot_path();
    let actual = render_snapshot();

    if std::env::var_os("UPDATE_SNAPSHOT").is_some() {
        fs::write(&snapshot_path, &actual).expect("cannot write snapshot");
        return;
    }

    let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|_| {
        panic!(
            "{} is missing — create it with UPDATE_SNAPSHOT=1 cargo test -p maiscope-viewer",
            snapshot_path.display()
        )
    });

    assert_eq!(
        expected.trim(),
        actual.trim(),
        "\nevent counts changed. If this is intended:\n  \
         UPDATE_SNAPSHOT=1 cargo test -p maiscope-viewer\n"
    );
}
