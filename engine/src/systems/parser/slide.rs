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

    // Last segment must have a duration (simai spec). If only the last has one,
    // all segments share it (shared_duration = true).
    let last_duration = segments
        .last()
        .and_then(|s| s.duration)
        .ok_or_else(|| format!("Slide requires duration: '{}'", pattern_str))?;

    let shared_duration = segments.iter().any(|s| s.duration.is_none());

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
            kind,
        });
    }

    Ok(notes)
}
