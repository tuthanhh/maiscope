//! Chart-file parsing: read the file, tokenize on commas, strip meta tokens
//! (BPM / resolution / absolute-length), and dispatch each note group.

use super::note::parse_note;
use crate::systems::component::{ChartEvent, Note};
use regex::Regex;
use std::sync::LazyLock;

// Pre-compiled regexes for performance (avoids recompilation on every call)
static BPM_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((\d+(\.\d+)?)\)").unwrap());
static RES_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\d+)\}").unwrap());
static ABS_LEN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{#(\d+(\.\d+)?)\}").unwrap());

/// Parse a simai chart file and return a sequence of chart events
pub fn parse_chart(inp: &str) -> Result<Vec<ChartEvent>, std::io::Error> {
    let content = inp.to_string();
    let clean: String = content.chars().filter(|c| !c.is_whitespace()).collect();

    let tokens: Vec<&str> = clean.split(',').collect();
    let mut events: Vec<ChartEvent> = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        let (current_str, meta) = strip_meta_tokens(token).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("token {idx}: {e}"))
        })?;
        events.extend(meta);

        if current_str.is_empty() {
            events.push(ChartEvent::Rest);
            continue;
        }

        // A bare "E" is the chart-end marker, not a Touch note.
        if current_str == "E" {
            events.push(ChartEvent::Rest);
            continue;
        }

        let notes_result: Result<Vec<Note>, String> = current_str
            .split('/')
            .map(parse_note)
            .collect::<Result<Vec<Vec<Note>>, String>>()
            .map(|v| v.into_iter().flatten().collect());

        match notes_result {
            Ok(notes) => events.push(ChartEvent::NoteGroup(notes)),
            Err(e) => eprintln!(
                "Warning: Error parsing note at token {}: '{}' - {}",
                idx, token, e
            ),
        }
    }

    println!("Parsed {} events", events.len());
    Ok(events)
}

// Strip the BPM, resolution and absolute-length markers from one token,
// returning the remaining note text plus the events those markers produce.
//
// Markers may be written anywhere in the token and in any order, so the loop
// runs until none remain. What comes back is *not* in the order they were
// written: a token emits at most one BPM change and at most one length change,
// always BPM first, then the length, then (by the caller) the note itself.
// Normalising is safe because a length divider cannot be interpreted without a
// BPM — the notation requires the BPM to be defined first regardless.
//
// Each kind may appear at most once per token. `(120)(240)1` and `{8}{16}1`
// have no defined meaning: the parser would apply one and silently discard the
// other, and which one survived would be an artefact of the matching order
// rather than of anything the chart author wrote. `{N}` and `{#S}` count as the
// *same* kind — both set the per-comma length, and `{#S}` explicitly replaces
// the divider — so `{8}{#0.35}1` is rejected too.
//
// The `{#S}`-before-`{N}` check order is arbitrary and carries no correctness
// weight: `RES_REGEX` requires a digit directly after `{`, so it cannot match
// `{#0.35}` at all. See
// `tests::absolute_length_is_not_mistaken_for_a_resolution_change`.
fn strip_meta_tokens(token: &str) -> Result<(String, Vec<ChartEvent>), String> {
    let mut rest = token.to_string();
    let mut bpm: Option<ChartEvent> = None;
    let mut length: Option<ChartEvent> = None;

    loop {
        let mut found = false;

        if let Some(caps) = ABS_LEN_REGEX.captures(&rest)
            && let Ok(seconds) = caps[1].parse::<f64>()
        {
            reject_duplicate(length.is_some(), token, Marker::Length)?;
            length = Some(ChartEvent::AbsoluteLength(seconds));
            let m = caps.get(0).unwrap();
            rest = format!("{}{}", &rest[..m.start()], &rest[m.end()..]);
            found = true;
        }

        if let Some(caps) = BPM_REGEX.captures(&rest)
            && let Ok(value) = caps[1].parse::<f32>()
        {
            reject_duplicate(bpm.is_some(), token, Marker::Bpm)?;
            bpm = Some(ChartEvent::BpmChange(value));
            let m = caps.get(0).unwrap();
            rest = format!("{}{}", &rest[..m.start()], &rest[m.end()..]);
            found = true;
        }

        if let Some(caps) = RES_REGEX.captures(&rest)
            && let Ok(resolution) = caps[1].parse::<u32>()
        {
            reject_duplicate(length.is_some(), token, Marker::Length)?;
            length = Some(ChartEvent::ResolutionChange(resolution));
            let m = caps.get(0).unwrap();
            rest = format!("{}{}", &rest[..m.start()], &rest[m.end()..]);
            found = true;
        }

        if !found {
            break;
        }
    }

    let mut events = Vec::with_capacity(2);
    events.extend(bpm);
    events.extend(length);
    Ok((rest, events))
}

// Which marker slot a duplicate was found in, so the error can name it.
#[derive(Clone, Copy)]
enum Marker {
    Bpm,
    Length,
}

fn reject_duplicate(already_set: bool, token: &str, marker: Marker) -> Result<(), String> {
    if !already_set {
        return Ok(());
    }
    let (name, spellings) = match marker {
        Marker::Bpm => ("BPM", "(B)"),
        Marker::Length => ("length", "{N} or {#S}"),
    };
    Err(format!(
        "token '{token}' carries more than one {name} marker ({spellings}); \
         at most one per token, since the second would silently overwrite the first"
    ))
}

#[cfg(test)]
mod tests {
    use super::super::testutil::{plain, simple};
    use super::*;
    use crate::systems::component::NoteKind;

    /// Parse a whole chart, panicking if it was rejected.
    fn events(chart: &str) -> Vec<ChartEvent> {
        parse_chart(chart).unwrap_or_else(|e| panic!("parse_chart failed: {e}"))
    }

    /// Assert the chart was rejected, and hand back the message to check.
    fn parse_err(chart: &str) -> String {
        match parse_chart(chart) {
            Err(e) => e.to_string(),
            Ok(events) => panic!("expected {chart:?} to be rejected, got {events:#?}"),
        }
    }

    /// A one-note group holding a plain tap — the building block of most
    /// expected sequences below.
    fn tap(button: usize) -> ChartEvent {
        ChartEvent::NoteGroup(vec![plain(NoteKind::Tap(button))])
    }

    /// A group of simultaneous taps, as `1/5,` produces.
    fn taps(buttons: &[usize]) -> ChartEvent {
        ChartEvent::NoteGroup(buttons.iter().map(|b| plain(NoteKind::Tap(*b))).collect())
    }

    #[test]
    fn commas_split_tokens_and_whitespace_is_ignored() {
        assert_eq!(
            events("1,2,3,"),
            vec![tap(1), tap(2), tap(3), ChartEvent::Rest]
        );
    }

    // --- Tokenization -------------------------------------------------------

    /// An empty token is a `Rest`, and so is a bare `E` — the chart-end marker,
    /// deliberately *not* parsed as touch zone E.
    ///
    /// Rests carry the beat grid: every comma occupies a fixed length of time
    /// whether or not a note sits on it. A missing `Rest` shifts every note
    /// after it, which is the silent-failure mode this whole ticket exists for.
    #[test]
    fn empty_tokens_and_the_end_marker_are_rests() {
        // ",," is three tokens, all empty.
        assert_eq!(
            events(",,"),
            vec![ChartEvent::Rest, ChartEvent::Rest, ChartEvent::Rest]
        );
        assert_eq!(events("E"), vec![ChartEvent::Rest]);
        assert_eq!(
            events("1,,2,"),
            vec![tap(1), ChartEvent::Rest, tap(2), ChartEvent::Rest]
        );

        // A trailing `E` after the last comma, as every real chart ends.
        assert_eq!(events("1,2,E"), vec![tap(1), tap(2), ChartEvent::Rest]);
    }

    /// `E` is only the end marker when it is the *entire* token. Inside a `/`
    /// group it is still sensor group E, because `parse_chart` compares the
    /// whole token before splitting.
    #[test]
    fn end_marker_only_applies_to_a_whole_token() {
        assert_eq!(
            events("E3"),
            vec![ChartEvent::NoteGroup(vec![plain(NoteKind::Touch {
                value: 3,
                group: 'E',
            })])]
        );
        assert_eq!(
            events("1/E"),
            vec![ChartEvent::NoteGroup(vec![
                plain(NoteKind::Tap(1)),
                plain(NoteKind::Touch {
                    value: 1,
                    group: 'E',
                }),
            ])]
        );
    }

    /// `/` separates the components of an EACH inside one token, and they all
    /// land in a single `NoteGroup` — `parse_chart` flattens a `Vec<Vec<Note>>`.
    ///
    /// Order is preserved, which matters: the notation doc says that for slides
    /// in an EACH, whichever is written first is displayed as occurring first.
    #[test]
    fn slash_groups_simultaneous_notes_into_one_event() {
        assert_eq!(events("1/5,"), vec![taps(&[1, 5]), ChartEvent::Rest]);
        assert_eq!(events("1/5/3,"), vec![taps(&[1, 5, 3]), ChartEvent::Rest]);

        // The doc's mixed example: a tap plus a hold.
        assert_eq!(
            events("1/8h[2:1]"),
            vec![ChartEvent::NoteGroup(vec![
                plain(NoteKind::Tap(1)),
                plain(NoteKind::TapHold {
                    button: 8,
                    duration: simple(2, 1),
                }),
            ])]
        );
    }

    /// Line breaks, spaces and tabs may be inserted anywhere for readability
    /// and are ignored during parsing. This is the property that lets fixtures
    /// be formatted legibly, so it guards `tests/fixtures/` as much as the parser.
    #[test]
    fn multiline_chart_equals_its_single_line_form() {
        let multiline = "(120){8}\n\
                         1, 2,\n\
                         \t3/5,\n\
                         E";
        assert_eq!(events(multiline), events("(120){8}1,2,3/5,E"));

        // Whitespace inside a token is stripped too, not just between them.
        assert_eq!(events("1 - 5 [ 8 : 1 ]"), events("1-5[8:1]"));
    }

    // --- Meta tokens --------------------------------------------------------

    /// A marker is stripped from its token and pushed as its own event *ahead*
    /// of the note that shared the token.
    #[test]
    fn bpm_marker_precedes_the_note_in_its_token() {
        assert_eq!(
            events("(120)1,"),
            vec![ChartEvent::BpmChange(120.0), tap(1), ChartEvent::Rest]
        );

        // BPM has to be exact, so decimals are part of the notation.
        assert_eq!(
            events("(174.5)1"),
            vec![ChartEvent::BpmChange(174.5), tap(1)]
        );
    }

    /// BPM may be redefined anywhere in the chart. The second `BpmChange` lands
    /// *between* the events for tokens 2 and 3 — downstream timing depends on
    /// that position, not just on the value, so assert the whole sequence.
    #[test]
    fn bpm_changes_mid_chart() {
        assert_eq!(
            events("(120)1,2,(240)3,4,"),
            vec![
                ChartEvent::BpmChange(120.0),
                tap(1),
                tap(2),
                ChartEvent::BpmChange(240.0),
                tap(3),
                tap(4),
                ChartEvent::Rest,
            ]
        );
    }

    /// `{8}` sets the length divider; `{#0.35}` sets the per-comma length
    /// directly in seconds.
    ///
    /// The comment in `strip_meta_tokens` used to claim the `{#S}`-before-`{N}`
    /// check order was load-bearing — that `{#0.35}` would otherwise be
    /// half-eaten by the integer-only resolution pattern. It would not:
    /// `RES_REGEX` needs a digit directly after `{`, so it cannot match at all.
    /// The two patterns are disjoint and the check order is free.
    #[test]
    fn absolute_length_is_not_mistaken_for_a_resolution_change() {
        assert_eq!(
            events("{8}1,"),
            vec![ChartEvent::ResolutionChange(8), tap(1), ChartEvent::Rest]
        );
        assert_eq!(
            events("{#0.35}1,"),
            vec![ChartEvent::AbsoluteLength(0.35), tap(1), ChartEvent::Rest,]
        );

        // The disjointness the comment above depends on.
        assert!(RES_REGEX.captures("{#0.35}").is_none());
        assert!(RES_REGEX.captures("{#35}").is_none());
    }

    /// A token emits its markers in a fixed order — BPM, then the length
    /// marker, then the note — whatever order they were written in.
    ///
    /// The notation doc requires BPM before the divider, since a note length
    /// cannot be computed without a BPM. Rather than reject the reversed
    /// spelling, the parser normalises it, so both forms produce the same
    /// sequence.
    ///
    /// Compared against an explicit expected vector, not just `a == b`:
    /// equality alone would still hold if both spellings started emitting the
    /// wrong order together.
    #[test]
    fn markers_combine_in_any_order() {
        let expected = vec![
            ChartEvent::BpmChange(120.0),
            ChartEvent::ResolutionChange(8),
            tap(1),
            ChartEvent::Rest,
        ];
        assert_eq!(events("(120){8}1,"), expected);
        assert_eq!(events("{8}(120)1,"), expected);

        // Same rule with the other length marker.
        assert_eq!(
            events("{#0.35}(120)1"),
            vec![
                ChartEvent::BpmChange(120.0),
                ChartEvent::AbsoluteLength(0.35),
                tap(1),
            ]
        );
    }

    /// One BPM and one length marker is the most a single token may carry, and
    /// both are stripped before the note is parsed.
    #[test]
    fn a_token_may_carry_one_marker_of_each_kind() {
        assert_eq!(
            events("(120){#0.35}1"),
            vec![
                ChartEvent::BpmChange(120.0),
                ChartEvent::AbsoluteLength(0.35),
                tap(1),
            ]
        );
    }

    // --- Duplicate markers --------------------------------------------------

    /// Two BPM markers in one token is an error, not a last-one-wins.
    ///
    /// There is no reading of `(120)(240)1` under which one of the two is
    /// meant: whichever survived would be an artefact of the matching order,
    /// and the chart would play at a tempo its author never wrote.
    #[test]
    fn duplicate_bpm_in_one_token_is_an_error() {
        let err = parse_err("(120)(240)1");
        assert!(err.contains("BPM"), "{err}");

        // Position within the token does not launder it.
        parse_err("(120)1(240)");
        parse_err("1,(120)(240)2,");
    }

    /// `{N}` and `{#S}` are the same kind of marker — both set the per-comma
    /// length — so two of either, or one of each, is an error.
    ///
    /// `{8}{#0.35}` is the case that motivated this. It used to parse, emitting
    /// `AbsoluteLength` followed by `ResolutionChange`, and
    /// `chart_playback.rs:62-65` has `ResolutionChange` clear the absolute
    /// length. The 0.35s was silently discarded no matter which order the
    /// author wrote the two markers in.
    #[test]
    fn duplicate_length_marker_in_one_token_is_an_error() {
        for chart in [
            "{8}{16}1",      // two dividers
            "{#0.3}{#0.4}1", // two absolute lengths
            "{8}{#0.35}1",   // divider then absolute
            "{#0.35}{8}1",   // absolute then divider
        ] {
            let err = parse_err(chart);
            assert!(err.contains("length"), "{chart:?} gave: {err}");
        }
    }

    /// The restriction is per token, not per chart — redefining either marker
    /// in a *later* token is ordinary notation and stays legal.
    #[test]
    fn the_same_marker_in_later_tokens_is_fine() {
        assert_eq!(
            events("(120)1,(240)2,"),
            vec![
                ChartEvent::BpmChange(120.0),
                tap(1),
                ChartEvent::BpmChange(240.0),
                tap(2),
                ChartEvent::Rest,
            ]
        );
        assert_eq!(
            events("{8}1,{16}2,"),
            vec![
                ChartEvent::ResolutionChange(8),
                tap(1),
                ChartEvent::ResolutionChange(16),
                tap(2),
                ChartEvent::Rest,
            ]
        );
        // Switching out of absolute-length mode via a later divider is exactly
        // what `chart_playback` documents, and is untouched by this rule.
        assert_eq!(
            events("{#0.35}1,{8}2,"),
            vec![
                ChartEvent::AbsoluteLength(0.35),
                tap(1),
                ChartEvent::ResolutionChange(8),
                tap(2),
                ChartEvent::Rest,
            ]
        );
    }

    // --- The error path -----------------------------------------------------

    /// BUG: a token that fails to parse is logged to stderr and *no event is
    /// pushed*, so the chart silently loses a beat.
    ///
    /// `"1,@@@,3,"` yields three events where four are expected, and every note
    /// after the bad token lands one beat early. `parse_chart` is typed
    /// `Result<Vec<ChartEvent>, io::Error>` but no path ever returns `Err`, so
    /// a caller has no way to know this happened.
    ///
    /// Not fixed here: the fix is a design choice — skip, emit `Rest` to hold
    /// the grid, or propagate `Err` and refuse the chart — and it wants its own
    /// ticket. `BIRTH.txt` hits this for real via the unsupported backtick, see
    /// `note::tests::pseudo_each_backtick_is_unsupported`.
    #[test]
    fn unparseable_token_drops_the_event_and_shifts_the_chart() {
        let evs = events("1,@@@,3,");
        assert_eq!(evs, vec![tap(1), tap(3), ChartEvent::Rest]);
        assert_eq!(
            evs.len(),
            3,
            "the bad token vanished; a grid-preserving parser would emit 4 events"
        );

        // The failure is invisible to the caller — still `Ok`.
        assert!(parse_chart("@@@").is_ok());
        assert_eq!(events("@@@"), vec![]);
    }

    /// Never panics, whatever it is fed. A panic in wasm takes the whole canvas
    /// down, so this is the single most important property in the module.
    ///
    /// The slicing in `strip_meta_tokens` and in `tokenize_slide_pattern` is the
    /// risk: both index by byte offset, and the latter walks forward looking for
    /// a `]` that may not be there.
    #[test]
    fn parse_chart_never_panics() {
        let long_token = "1".repeat(10_000);
        let many_commas = ",".repeat(10_000);

        for chart in [
            "",               // empty
            ",,,,",           // only commas
            "1-5[8:1",        // unterminated bracket
            "(120",           // unterminated BPM marker
            "{8",             // unterminated resolution marker
            "{#",             // truncated absolute length
            "*",              // stray star chain
            "/",              // lone separator
            "1//5",           // empty component in an EACH
            "日本語",         // non-ASCII
            "(120)日本(240)", // non-ASCII around byte-offset slicing
            "E,1,",           // notes after the end marker
            &long_token,
            &many_commas,
            "(120)(240)", // duplicate markers: rejected, must not panic
            "{8}{#0.35}",
        ] {
            // The property is "returns rather than unwinds". `Err` is a fine
            // outcome; a panic is not, so the assertion is simply that we got here.
            let _ = parse_chart(chart);
        }
    }
}
