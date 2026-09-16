//! Slide-note parsing: shape patterns, chained segments, and star chains.

use super::duration::parse_duration_bracket;
use crate::systems::component::{Duration, Note, NoteKind, SlideSegment, SlideShape};
use regex::Regex;
use std::sync::LazyLock;

// Start button + modifiers + the remaining slide pattern. Compiled once.
static SLIDE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
            ^([1-8])                                 # Start button (must be digit for slides)
            ([xfb@?!$]*)                             # Modifiers before slide
            (.+)                                     # Slide pattern (everything in between)
            $
        ",
    )
    .unwrap()
});

pub(super) fn parse_slide_note(note_str: &str) -> Result<Vec<Note>, String> {
    // Parse a note with slide patterns
    // Format: <button><modifiers><slide_pattern><target>[duration]<modifiers>
    // Examples: 1-5[8:1], 3b>6[16:9], 5V71[4:1], 1-4[8:1]*-6[8:1]

    let caps = SLIDE_RE
        .captures(note_str)
        .ok_or_else(|| format!("Invalid slide syntax: '{}'", note_str))?;

    let start_btn = caps[1].parse::<usize>().unwrap();
    let modifiers_before = &caps[2];
    let slide_pattern_str = &caps[3];

    // Collect break/ex/firework from the entire pattern string as well,
    // since a break slide has 'b' after the ']' like 1-4[8:3]b
    let all_modifiers = format!("{}{}", modifiers_before, slide_pattern_str);
    // 'b' = BREAK (per simai spec), 'x' = EX note
    let is_break = all_modifiers.contains('b');
    let is_ex = modifiers_before.contains('x');
    let is_firework = modifiers_before.contains('f');

    // Star-chained slide: multiple independent paths radiating from the same star.
    if slide_pattern_str.contains('*') {
        return parse_star_chained_slides(
            start_btn,
            slide_pattern_str,
            is_break,
            is_firework,
            is_ex,
        );
    }

    // A single continuous path, possibly chained (multiple shapes without '*').
    // Examples: 3-5v8[4:1], 2<4p3[2:1], 1-4q7-2[1:2], 3V17V13[2:1]
    let (raw_segments, shared_duration) =
        parse_chained_slide_segments(start_btn, slide_pattern_str)?;

    let segments = build_segments(raw_segments, is_break);
    Ok(vec![Note {
        is_break,
        is_firework,
        is_ex,
        offset_ms: 0,
        kind: NoteKind::Slide {
            head_button: start_btn,
            segments,
            shared_duration,
        },
    }])
}

/// Convert raw `(shape, duration)` pairs into `SlideSegment`s carrying the
/// note's break flag.
fn build_segments(raw: Vec<(SlideShape, Duration)>, is_break: bool) -> Vec<SlideSegment> {
    raw.into_iter()
        .map(|(shape, duration)| SlideSegment {
            shape,
            duration,
            is_break,
        })
        .collect()
}

/// Represents a parsed slide segment before creating the final SlideShape
struct SlideSegmentRaw {
    shape_char: String,
    target_digits: String,
    duration: Option<Duration>,
}

/// Parse a slide pattern string into one or more chained slide segments.
///
/// This handles:
/// - Simple slides: "-5[8:1]" (straight from start to 5)
/// - Grand V: "V35[4:1]" (V-shape with mid=3, end=5)
/// - Chained slides: "-5v8[4:1]" (straight to 5, then v-shape to 8)
/// - Chained Grand V: "V17V13[2:1]" (grandV mid=1 end=7, then grandV mid=1 end=3)
/// - Chained with individual durations: "-4[2:1]q7[2:1]-2[1:1]"
fn parse_chained_slide_segments(
    start_btn: usize,
    pattern_str: &str,
) -> Result<(Vec<(SlideShape, Duration)>, bool), String> {
    let clean_str = pattern_str.trim_end_matches(['b', 'x', 'f']);

    let segments =
        tokenize_slide_pattern(clean_str).map_err(|e| format!("{} in '{}'", e, pattern_str))?;

    if segments.is_empty() {
        return Err(format!(
            "No valid slide segments found in '{}'",
            pattern_str
        ));
    }

    // The notation allows exactly two shapes, and nothing between them:
    //
    //   1-4q7-2[1:2]              one bracket, on the last segment — the whole
    //                             path traces at one speed derived from it
    //   1-4[2:1]q7[2:1]-2[1:1]    a bracket on every segment — per-segment speeds
    //
    // "When doing this, every sub-track needs its own length — omitting one
    // causes an error." A chain that brackets some segments but not all is
    // neither shape: it used to fill the bare ones from the last segment and
    // trace at a speed the author never wrote, silently. Rejected instead, on
    // the same grounds as ADR-0012's duplicate markers.
    let last_duration = segments
        .last()
        .and_then(|s| s.duration)
        .ok_or_else(|| format!("Slide requires duration: '{}'", pattern_str))?;

    let bare = segments.iter().filter(|s| s.duration.is_none()).count();
    // Every segment but the last is bare → the single-bracket form.
    let shared_duration = bare == segments.len() - 1 && bare > 0;

    if bare > 0 && !shared_duration {
        return Err(format!(
            "Slide '{}' brackets some segments but not all: {} of {} segments \
             have no duration. Write one duration on the last segment for a \
             single tracing speed, or one on every segment.",
            pattern_str,
            bare,
            segments.len()
        ));
    }

    let mut result = Vec::new();
    let mut current_start = start_btn;
    for seg in &segments {
        let duration = seg.duration.unwrap_or(last_duration);
        let (shape, end_btn) =
            build_slide_shape(current_start, &seg.shape_char, &seg.target_digits)?;
        result.push((shape, duration));
        current_start = end_btn;
    }

    Ok((result, shared_duration))
}

// Walk `clean_str` character by character and produce a list of raw slide segments.
// Handles: "pp"/"qq" two-char shapes, single-char shapes, digit runs, and [...] brackets.
fn tokenize_slide_pattern(clean_str: &str) -> Result<Vec<SlideSegmentRaw>, String> {
    let chars: Vec<char> = clean_str.chars().collect();
    let mut segments = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == 'b' || ch == 'x' || ch == 'f' {
            i += 1;
            continue;
        }

        let shape_str: String;
        if i + 1 < chars.len()
            && ((ch == 'p' && chars[i + 1] == 'p') || (ch == 'q' && chars[i + 1] == 'q'))
        {
            shape_str = format!("{}{}", ch, chars[i + 1]);
            i += 2;
        } else if "-^v<>pqszVw".contains(ch) {
            shape_str = ch.to_string();
            i += 1;
        } else {
            i += 1;
            continue;
        }

        let mut target_digits = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            target_digits.push(chars[i]);
            i += 1;
        }

        if target_digits.is_empty() {
            return Err(format!("Slide shape '{}' has no target digits", shape_str));
        }

        while i < chars.len() && (chars[i] == 'b' || chars[i] == 'x' || chars[i] == 'f') {
            i += 1;
        }

        let duration = if i < chars.len() && chars[i] == '[' {
            let bracket_start = i;
            while i < chars.len() && chars[i] != ']' {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            parse_duration_bracket(&clean_str[bracket_start..i])
        } else {
            None
        };

        segments.push(SlideSegmentRaw {
            shape_char: shape_str,
            target_digits,
            duration,
        });
    }

    Ok(segments)
}

/// Build a SlideShape from a shape indicator and target digit(s).
/// Returns (SlideShape, end_button) so the caller knows where the next segment starts.
fn build_slide_shape(
    _start_btn: usize,
    shape_str: &str,
    target_digits: &str,
) -> Result<(SlideShape, usize), String> {
    match shape_str {
        // Both `V` and `v` are grand-V with two digits (mid+end); a single
        // digit is the simple V through the center.
        "V" | "v" => {
            if target_digits.len() == 2 {
                let mid_btn = target_digits[0..1]
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid V mid button: '{}'", target_digits))?;
                let end_btn = target_digits[1..2]
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid V end button: '{}'", target_digits))?;
                Ok((
                    SlideShape::GrandVShape {
                        mid: mid_btn,
                        end: end_btn,
                    },
                    end_btn,
                ))
            } else if target_digits.len() == 1 {
                // Single digit V treated as lowercase v
                let end_btn = parse_end_button(target_digits, "V")?;
                Ok((SlideShape::VShape { end: end_btn }, end_btn))
            } else {
                Err(format!(
                    "Invalid Grand V target digits: '{}'",
                    target_digits
                ))
            }
        }
        "-" => {
            let e = parse_end_button(target_digits, "straight")?;
            Ok((SlideShape::Straight { end: e }, e))
        }
        "^" => {
            let e = parse_end_button(target_digits, "arc")?;
            Ok((SlideShape::ShortArc { end: e }, e))
        }
        "<" => {
            let e = parse_end_button(target_digits, "CCW arc")?;
            Ok((SlideShape::CounterClockwiseArc { end: e }, e))
        }
        ">" => {
            let e = parse_end_button(target_digits, "CW arc")?;
            Ok((SlideShape::ClockwiseArc { end: e }, e))
        }
        "p" => {
            let e = parse_end_button(target_digits, "p")?;
            Ok((SlideShape::PShape { end: e }, e))
        }
        "q" => {
            let e = parse_end_button(target_digits, "q")?;
            Ok((SlideShape::QShape { end: e }, e))
        }
        "pp" => {
            let e = parse_end_button(target_digits, "pp")?;
            Ok((SlideShape::GrandPShape { end: e }, e))
        }
        "qq" => {
            let e = parse_end_button(target_digits, "qq")?;
            Ok((SlideShape::GrandQShape { end: e }, e))
        }
        "s" => {
            let e = parse_end_button(target_digits, "s")?;
            Ok((
                SlideShape::Thunderbolt {
                    end: e,
                    is_z: false,
                },
                e,
            ))
        }
        "z" => {
            let e = parse_end_button(target_digits, "z")?;
            Ok((SlideShape::Thunderbolt { end: e, is_z: true }, e))
        }
        "w" => {
            let e = parse_end_button(target_digits, "w")?;
            let e2 = if e >= 8 { 1 } else { e + 1 };
            let e3 = if e <= 1 { 8 } else { e - 1 };
            Ok((SlideShape::FanShape { ends: (e, e2, e3) }, e))
        }
        _ => Err(format!("Unknown slide shape: '{}'", shape_str)),
    }
}

fn parse_end_button(target_digits: &str, shape_name: &str) -> Result<usize, String> {
    target_digits
        .parse::<usize>()
        .map_err(|_| format!("Invalid {} target: '{}'", shape_name, target_digits))
}

/// Parse star-chained slides like "1-4[8:1]*-6[8:1]" into multiple notes.
///
/// Each `*`-separated path is independent and radiates from the same starting
/// star button. The first path carries the star head (`Slide`); the remaining
/// paths are headless (`HeadlessSlide`). A path may itself be chained, in which
/// case all of its shapes become segments of that one note.
fn parse_star_chained_slides(
    start_btn: usize,
    pattern_str: &str,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Result<Vec<Note>, String> {
    let paths: Vec<&str> = pattern_str.split('*').collect();
    let mut notes = Vec::with_capacity(paths.len());

    for (idx, path) in paths.iter().enumerate() {
        let (raw_segments, shared_duration) = parse_chained_slide_segments(start_btn, path)?;
        let segments = build_segments(raw_segments, is_break);

        let kind = if idx == 0 {
            NoteKind::Slide {
                head_button: start_btn,
                segments,
                shared_duration,
            }
        } else {
            NoteKind::HeadlessSlide {
                start_button: start_btn,
                segments,
                shared_duration,
            }
        };

        notes.push(Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind,
        });
    }

    Ok(notes)
}

#[cfg(test)]
mod tests {
    // `parse` and `parse_err` are imported for the stubs below, not for the
    // tests that already exist.
    #[allow(unused_imports)]
    use super::super::testutil::{parse, parse_err, parse_one, plain, seg, simple};
    use crate::systems::component::{NoteKind, SlideShape};

    /// Shorthand for the overwhelmingly common case: one slide note, one
    /// segment, no modifiers, not shared-duration.
    fn one_segment(head: usize, shape: SlideShape, divider: usize, count: usize) -> NoteKind {
        NoteKind::Slide {
            head_button: head,
            segments: vec![seg(shape, simple(divider, count), false)],
            shared_duration: false,
        }
    }

    #[test]
    fn straight_slide() {
        assert_eq!(
            parse_one("1-5[8:1]"),
            plain(one_segment(1, SlideShape::Straight { end: 5 }, 8, 1))
        );
    }

    // --- One test per shape -------------------------------------------------
    // `build_slide_shape` has twelve arms, each mapping a shape string to a
    // `SlideShape` variant. That mapping is the whole risk surface: a swapped
    // arm renders a plausible-but-wrong path and nobody reports it.

    /// The single-character shapes, per the track-shapes table in
    /// `docs/reference/simai-notation.md`.
    ///
    /// `>` and `<` are the pair worth checking rather than trusting. Buttons are
    /// numbered *clockwise*, and the doc defines `>` as rightward travel — which
    /// is the increasing direction, i.e. clockwise. The mapping agrees.
    #[test]
    fn single_char_shapes_map_to_their_variants() {
        for (input, shape) in [
            ("1^5[8:1]", SlideShape::ShortArc { end: 5 }),
            ("1>5[8:1]", SlideShape::ClockwiseArc { end: 5 }),
            ("1<5[8:1]", SlideShape::CounterClockwiseArc { end: 5 }),
            ("1p5[8:1]", SlideShape::PShape { end: 5 }),
            ("1q5[8:1]", SlideShape::QShape { end: 5 }),
        ] {
            assert_eq!(
                parse_one(input),
                plain(one_segment(1, shape, 8, 1)),
                "{input:?}"
            );
        }
    }

    /// `pp` / `qq` are Grand P / Grand Q — curves along a circle tangent to both
    /// the screen centre and the judgment line, not a doubled `p`.
    ///
    /// `tokenize_slide_pattern` special-cases them *before* its single-char
    /// branch, so this also pins that precedence: `1pp5[8:1]` must be one Grand P
    /// segment, not a P-shape followed by an orphaned `p`.
    #[test]
    fn grand_p_and_q_are_two_char_shapes() {
        assert_eq!(
            parse_one("1pp5[8:1]"),
            plain(one_segment(1, SlideShape::GrandPShape { end: 5 }, 8, 1))
        );
        assert_eq!(
            parse_one("1qq5[8:1]"),
            plain(one_segment(1, SlideShape::GrandQShape { end: 5 }, 8, 1))
        );
    }

    /// `s` and `z` are both the thunderbolt, distinguished only by `is_z`.
    #[test]
    fn thunderbolt_s_and_z_differ_only_by_flag() {
        assert_eq!(
            parse_one("1s5[8:1]"),
            plain(one_segment(
                1,
                SlideShape::Thunderbolt {
                    end: 5,
                    is_z: false,
                },
                8,
                1
            ))
        );
        assert_eq!(
            parse_one("1z5[8:1]"),
            plain(one_segment(
                1,
                SlideShape::Thunderbolt { end: 5, is_z: true },
                8,
                1
            ))
        );
    }

    /// `w` is the fan: one start expanding into three ends, spawning three
    /// tracing stars.
    ///
    /// Only one end is written; the other two are *derived* as `e+1` and `e-1`,
    /// wrapping at the 1/8 boundary. Both wraps are asserted — a naive `e+1`
    /// would give 9 for `end: 8`, and a naive `e-1` would give 0 for `end: 1`,
    /// neither of which is a button.
    #[test]
    fn fan_shape_ends_wrap_around_the_ring() {
        let cases = [
            ("1w4[8:1]", (4, 5, 3)), // middle of the range
            ("1w8[8:1]", (8, 1, 7)), // upper wrap: 8+1 is 1, not 9
            ("2w1[8:1]", (1, 2, 8)), // lower wrap: 1-1 is 8, not 0
        ];
        for (input, ends) in cases {
            let head = input[..1].parse::<usize>().unwrap();
            assert_eq!(
                parse_one(input),
                plain(one_segment(head, SlideShape::FanShape { ends }, 8, 1)),
                "{input:?}"
            );
        }
    }

    /// `V` is overloaded on how many target digits follow it: two digits name a
    /// turning point and an end (`1V35` turns at 3, ends at 5), one digit is the
    /// simple V through the screen centre.
    #[test]
    fn v_shape_arity_selects_grand_or_simple() {
        assert_eq!(
            parse_one("1V35[8:1]"),
            plain(one_segment(
                1,
                SlideShape::GrandVShape { mid: 3, end: 5 },
                8,
                1
            ))
        );
        assert_eq!(
            parse_one("1V5[8:1]"),
            plain(one_segment(1, SlideShape::VShape { end: 5 }, 8, 1))
        );

        // Three digits names nothing.
        let err = parse_err("1V357[8:1]");
        assert!(err.contains("Grand V"), "{err}");
    }

    /// Divergence: `v` and `V` share one arm, so arity alone decides the shape.
    ///
    /// The notation doc gives them distinct meanings — `v` is always the
    /// centre-turning V, `V` the grand V with an explicit turning point — so
    /// `1v35[8:1]` should arguably be an error rather than a grand V. It is not
    /// notation anyone writes, and treating the two spellings alike is harmless
    /// for the single-digit case that matters, so this pins the behaviour rather
    /// than calling it a defect.
    #[test]
    fn lowercase_v_is_treated_as_grand_v_when_given_two_digits() {
        assert_eq!(parse_one("1v5[8:1]"), parse_one("1V5[8:1]"));
        assert_eq!(parse_one("1v35[8:1]"), parse_one("1V35[8:1]"));
    }

    // --- Chaining -----------------------------------------------------------

    /// A chaining slide joins tracks end-to-start into one continuous path, so
    /// each segment starts where the previous one ended — `current_start` in
    /// `parse_chained_slide_segments` threads that forward.
    ///
    /// `3-5v8[4:1]` is 3→5 straight, then 5→8 through the centre. The `8` is a
    /// target, not a start; nothing in the text says the second segment begins
    /// at 5, which is exactly why this needs pinning.
    #[test]
    fn chained_segments_thread_the_end_button_forward() {
        assert_eq!(
            parse_one("3-5v8[4:1]"),
            plain(NoteKind::Slide {
                head_button: 3,
                segments: vec![
                    seg(SlideShape::Straight { end: 5 }, simple(4, 1), false),
                    seg(SlideShape::VShape { end: 8 }, simple(4, 1), false),
                ],
                shared_duration: true,
            })
        );

        // The doc's three-segment example: 1→4 straight, 4→7 Q, 7→2 straight.
        assert_eq!(
            parse_one("1-4q7-2[1:2]"),
            plain(NoteKind::Slide {
                head_button: 1,
                segments: vec![
                    seg(SlideShape::Straight { end: 4 }, simple(1, 2), false),
                    seg(SlideShape::QShape { end: 7 }, simple(1, 2), false),
                    seg(SlideShape::Straight { end: 2 }, simple(1, 2), false),
                ],
                shared_duration: true,
            })
        );
    }

    /// `shared_duration` records *how the duration was written*, not whether the
    /// segments happen to agree.
    ///
    /// Only the last segment carrying a bracket means one constant speed over
    /// the whole path, computed from the total length — the flag is `true` and
    /// every segment inherits that duration. Each segment carrying its own means
    /// the speeds may differ, and the flag is `false`.
    ///
    /// Note the asymmetry with a *single*-segment slide: `1-5[8:1]` has its own
    /// bracket, so the flag is `false` there too. "Shared" means "at least one
    /// segment inherited", not "all segments match".
    #[test]
    fn shared_duration_flag_tracks_whether_brackets_were_omitted() {
        let inherited = parse_one("1-4q7[8:1]");
        match &inherited.kind {
            NoteKind::Slide {
                segments,
                shared_duration,
                ..
            } => {
                assert!(shared_duration, "only the last bracket was written");
                assert_eq!(segments.len(), 2);
                assert!(segments.iter().all(|s| s.duration == simple(8, 1)));
            }
            other => panic!("expected a Slide, got {other:?}"),
        }

        let per_segment = parse_one("1-4[2:1]q7[2:1]-2[1:1]");
        match &per_segment.kind {
            NoteKind::Slide {
                segments,
                shared_duration,
                ..
            } => {
                assert!(!shared_duration, "every segment wrote its own bracket");
                assert_eq!(segments.len(), 3);
                assert_eq!(segments[0].duration, simple(2, 1));
                assert_eq!(segments[1].duration, simple(2, 1));
                assert_eq!(segments[2].duration, simple(1, 1));
            }
            other => panic!("expected a Slide, got {other:?}"),
        }

        // A single segment with its own bracket is not "shared".
        match parse_one("1-5[8:1]").kind {
            NoteKind::Slide {
                shared_duration, ..
            } => assert!(!shared_duration),
            other => panic!("expected a Slide, got {other:?}"),
        }
    }

    /// A chain that brackets some segments but not all is rejected.
    ///
    /// The notation allows exactly two shapes — one bracket on the last segment,
    /// or a bracket on every segment. "When doing this, every sub-track needs
    /// its own length — omitting one causes an error."
    ///
    /// `1-4[2:1]q7-2[1:1]` is neither. It used to fill the bare middle segment
    /// from the last one and trace at a speed nobody wrote, with nothing logged.
    #[test]
    fn partially_bracketed_chain_is_an_error() {
        let err = parse_err("1-4[2:1]q7-2[1:1]");
        assert!(err.contains("some segments but not all"), "{err}");
        assert!(err.contains("1 of 3"), "{err}");

        // First bracketed, rest bare — also neither shape.
        parse_err("1-4[2:1]q7-2[1:1]");
        parse_err("1-4[2:1]q7[2:1]-2");
    }

    /// A slide needs a tracing length; the last segment must carry one.
    #[test]
    fn slide_without_duration_is_an_error() {
        let err = parse_err("1-5");
        assert!(err.contains("duration"), "{err}");
        assert!(parse_err("1-4q7").contains("duration"));
    }

    // --- Star chains --------------------------------------------------------

    /// A multiple slide is one star-shaped TAP with several independent tracks,
    /// written `*`-separated. Only the starting point is shared.
    ///
    /// The first path carries the star (`Slide`); the rest are `HeadlessSlide`
    /// anchored at the same button. That split is what stops the visualizer
    /// drawing one star per track.
    #[test]
    fn star_chain_gives_one_head_and_headless_rest() {
        assert_eq!(
            parse("1-4[4:3]*-6[8:5]"),
            vec![
                plain(NoteKind::Slide {
                    head_button: 1,
                    segments: vec![seg(SlideShape::Straight { end: 4 }, simple(4, 3), false)],
                    shared_duration: false,
                }),
                plain(NoteKind::HeadlessSlide {
                    start_button: 1,
                    segments: vec![seg(SlideShape::Straight { end: 6 }, simple(8, 5), false)],
                    shared_duration: false,
                }),
            ]
        );

        // Adding another `*` adds another headless track; only index 0 is a head.
        let three = parse("1-4[8:1]*-6[8:1]*-2[8:1]");
        assert_eq!(three.len(), 3);
        assert!(matches!(three[0].kind, NoteKind::Slide { .. }));
        assert!(matches!(three[1].kind, NoteKind::HeadlessSlide { .. }));
        assert!(matches!(three[2].kind, NoteKind::HeadlessSlide { .. }));
    }

    /// The two features compose: a path inside a star chain may itself be a
    /// chain. `-6v2[8:1]` is one headless note with two segments, not two notes.
    #[test]
    fn star_chain_path_may_itself_be_chained() {
        let notes = parse("1-4[8:1]*-6v2[8:1]");
        assert_eq!(notes.len(), 2);
        assert_eq!(
            notes[1],
            plain(NoteKind::HeadlessSlide {
                start_button: 1,
                segments: vec![
                    seg(SlideShape::Straight { end: 6 }, simple(8, 1), false),
                    seg(SlideShape::VShape { end: 2 }, simple(8, 1), false),
                ],
                shared_duration: true,
            })
        );
    }

    // --- Modifiers ----------------------------------------------------------

    /// A BREAK slide writes `b` after the closing bracket. `is_break` is scraped
    /// from the whole pattern string and `build_segments` copies it onto every
    /// segment — matching the doc's rule that a chaining slide is entirely
    /// normal or entirely BREAK, never partial.
    #[test]
    fn break_propagates_to_every_segment() {
        let note = parse_one("1-4[8:3]b");
        assert!(note.is_break);
        match &note.kind {
            NoteKind::Slide { segments, .. } => {
                assert!(segments.iter().all(|s| s.is_break));
            }
            other => panic!("expected a Slide, got {other:?}"),
        }

        // A chained break marks all of them.
        match &parse_one("1-4q7[8:1]b").kind {
            NoteKind::Slide { segments, .. } => {
                assert_eq!(segments.len(), 2);
                assert!(segments.iter().all(|s| s.is_break));
            }
            other => panic!("expected a Slide, got {other:?}"),
        }
    }

    /// Asymmetry worth knowing: `is_break` is read from the entire pattern, but
    /// `is_ex` and `is_firework` come from `modifiers_before` only.
    ///
    /// So `1x-4[8:1]` is an EX slide and `1-4[8:1]x` is not — it parses happily
    /// and drops the flag. The doc says `x` is "added the same way BREAK's `b`
    /// is added", but it gives no EX-slide example, and attaching `x` to the
    /// star head (before the pattern) is the reading that matches how the star
    /// is what gets judged. Pinned rather than labelled a defect; if a real chart
    /// ever writes the trailing form, this test is where the decision is recorded.
    #[test]
    fn ex_and_firework_bind_only_before_the_pattern() {
        assert!(parse_one("1x-4[8:1]").is_ex);
        assert!(!parse_one("1-4[8:1]x").is_ex);

        assert!(parse_one("1f-4[8:1]").is_firework);
        assert!(!parse_one("1-4[8:1]f").is_firework);

        // Trailing `x` is still stripped from the pattern, not treated as text.
        assert_eq!(
            parse_one("1-4[8:1]x").kind,
            one_segment(1, SlideShape::Straight { end: 4 }, 8, 1)
        );
    }

    /// The star-suppression and normal-TAP modifiers parse and are then ignored.
    ///
    /// `1?-5[2:1]` fades the tracing star in, `1!-5[2:1]` pops it in, `1@-5[2:1]`
    /// reverts the head to a normal TAP. `SLIDE_RE` admits all three and nothing
    /// downstream reads them, so all four spellings produce an identical note.
    ///
    /// Contrast `note.rs`, where `$` and `@` on a *tap* fail outright — the gap
    /// is inconsistent rather than uniform. See
    /// `note::tests::utage_star_tap_modifiers_are_unsupported`.
    #[test]
    fn star_suppression_modifiers_parse_but_are_ignored() {
        let plain_slide = parse_one("1-5[2:1]");
        for input in ["1?-5[2:1]", "1!-5[2:1]", "1@-5[2:1]"] {
            assert_eq!(parse_one(input), plain_slide, "{input:?}");
        }
    }

    // --- Rejection ----------------------------------------------------------

    /// Returns `Err`, never panics.
    ///
    /// The unterminated bracket is the one to watch: `tokenize_slide_pattern`
    /// scans forward for a `]` and walks off the end when there is none, then
    /// slices `clean_str[bracket_start..i]`. It survives because the slice is
    /// byte-bounded by the string itself and the malformed bracket simply fails
    /// to parse, leaving the segment with no duration.
    #[test]
    fn malformed_slides_return_err_never_panic() {
        for input in [
            "1-[8:1]",   // shape with no target
            "1v[8:1]",   // same, V arm
            "1w[8:1]",   // same, fan arm
            "1-5[8:1",   // unterminated bracket
            "1-5[]",     // empty bracket
            "1-5[a:b]",  // unparseable bracket
            "A1-5[8:1]", // non-digit start
            "9-5[8:1]",  // start outside 1-8
            "-5[8:1]",   // no start at all
            "1*",        // star chain with no paths
            "1-4[8:1]*", // star chain with an empty trailing path
        ] {
            parse_err(input);
        }
    }
}
