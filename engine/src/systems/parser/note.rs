//! Note-level parsing: dispatch to slide vs. tap/touch, and build tap, hold,
//! touch, and touch-hold notes.

use super::slide::parse_slide_note;
use crate::systems::component::{Duration, Note, NoteKind};
use regex::Regex;
use std::sync::LazyLock;

// Matches any simai slide shape character. A leading button digit guarantees a
// slide context, so these never collide with touch zones (A-E) or modifiers.
static SLIDE_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(pp|qq|[-^v<>pqszVw*])").unwrap());

// Button or touch location, modifiers, optional hold duration, trailing modifiers.
static TAP_TOUCH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
            ^([A-Ea-e][1-8]|[1-8]|[1-8][1-8]|[CEce])  # Button or touch location (zones case-insensitive)
            ([xhfb]*)                            # Modifiers
            (?:\[(\d+):(\d+)\])?                 # Optional hold duration
            ([xhfb]*)                            # Optional modifiers after duration
            $
        ",
    )
    .unwrap()
});

pub fn parse_note(note_str: &str) -> Result<Vec<Note>, String> {
    if note_str.is_empty() {
        return Err("Empty note string".to_string());
    }

    // Detect slides by their shape characters. Touch notes (e.g. "E3", "B5f")
    // contain none of these, so they fall through to the tap/touch parser.
    if SLIDE_PATTERN_RE.is_match(note_str) {
        return parse_slide_note(note_str);
    }

    parse_tap_or_touch_note(note_str)
}

fn parse_tap_or_touch_note(note_str: &str) -> Result<Vec<Note>, String> {
    // Modifiers: b=break, x=ex, h=hold, f=firework
    // Hold without duration is a pseudo-hold, treated as [1280:1] per spec.
    let caps = TAP_TOUCH_RE
        .captures(note_str)
        .ok_or_else(|| format!("Invalid note syntax: '{}'", note_str))?;

    let raw_loc = caps[1].to_string();
    let modifiers = caps[2].to_string();
    let modifiers_after = caps.get(5).map(|m| m.as_str()).unwrap_or("").to_string();
    let hold_duration = caps.get(3).zip(caps.get(4)).map(|(d, l)| {
        (
            d.as_str().parse().unwrap_or(1),
            l.as_str().parse().unwrap_or(1),
        )
    });

    let (is_break, is_ex, is_firework, is_hold) =
        parse_note_modifiers(&modifiers, &modifiers_after);

    // Two-digit shorthand EACH notation (e.g. "12" = buttons 1 and 2 simultaneously)
    if raw_loc.len() == 2 && raw_loc.chars().all(|c| c.is_ascii_digit()) {
        let chars: Vec<char> = raw_loc.chars().collect();
        let btn1 = chars[0].to_digit(10).unwrap_or(0) as usize;
        let btn2 = chars[1].to_digit(10).unwrap_or(0) as usize;
        return Ok(build_two_digit_tap_notes(
            btn1,
            btn2,
            is_break,
            is_firework,
            is_ex,
        ));
    }

    if let Ok(btn_num) = raw_loc.parse::<usize>() {
        return Ok(build_button_note(
            btn_num,
            is_hold,
            hold_duration,
            is_break,
            is_firework,
            is_ex,
        ));
    }

    let chars: Vec<char> = raw_loc.chars().collect();
    let zone = chars[0].to_ascii_uppercase();
    let index = chars.get(1).and_then(|c| c.to_digit(10)).unwrap_or(1) as usize;
    Ok(build_touch_note(
        zone,
        index,
        is_hold,
        hold_duration,
        is_break,
        is_firework,
        is_ex,
    ))
}

// Returns (is_break, is_ex, is_firework, is_hold) from the two modifier strings.
fn parse_note_modifiers(modifiers: &str, modifiers_after: &str) -> (bool, bool, bool, bool) {
    let is_break = modifiers.contains('b') || modifiers_after.contains('b');
    let is_ex = modifiers.contains('x') || modifiers_after.contains('x');
    let is_firework = modifiers.contains('f') || modifiers_after.contains('f');
    let is_hold = modifiers.contains('h') || modifiers_after.contains('h');
    (is_break, is_ex, is_firework, is_hold)
}

fn build_two_digit_tap_notes(
    btn1: usize,
    btn2: usize,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Vec<Note> {
    vec![
        Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::Tap(btn1),
        },
        Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::Tap(btn2),
        },
    ]
}

fn build_button_note(
    btn_num: usize,
    is_hold: bool,
    hold_duration: Option<(usize, usize)>,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Vec<Note> {
    if is_hold {
        // Pseudo-hold (no duration) is treated as [1280:1] per spec.
        let (divider, count) = hold_duration.unwrap_or((1280, 1));
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::TapHold {
                button: btn_num,
                duration: Duration::Simple { divider, count },
            },
        }]
    } else {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::Tap(btn_num),
        }]
    }
}

fn build_touch_note(
    zone: char,
    index: usize,
    is_hold: bool,
    hold_duration: Option<(usize, usize)>,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Vec<Note> {
    if is_hold {
        // Pseudo touch-hold (no duration) is treated as [1280:1] per spec.
        let (divider, count) = hold_duration.unwrap_or((1280, 1));
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::TouchHold {
                value: index,
                group: zone,
                duration: Duration::Simple { divider, count },
            },
        }]
    } else {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            kind: NoteKind::Touch {
                value: index,
                group: zone,
            },
        }]
    }
}
