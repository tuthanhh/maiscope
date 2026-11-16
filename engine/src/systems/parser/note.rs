//! Note-level parsing: dispatch to slide vs. tap/touch, and build tap, hold,
//! touch, and touch-hold notes.

use super::duration::parse_duration_bracket;
use super::slide::parse_slide_note;
use crate::systems::component::{Duration, Note, NoteKind};
use regex::Regex;
use std::sync::LazyLock;

// Matches any simai slide shape character. A leading button digit guarantees a
// slide context, so these never collide with touch zones (A-E) or modifiers.
static SLIDE_PATTERN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(pp|qq|[-^v<>pqszVw*])").unwrap());

// Button or touch location, modifiers, optional hold duration, trailing modifiers.
//
// The duration group captures the whole bracket rather than picking out `N:M`,
// so every form `parse_duration_bracket` understands reaches it — the notation
// allows `4h[#5.678]` and `4h[150#2:1]` as well as `5h[2:1]`. Matching the
// bracket loosely here and validating it there keeps one parser for durations
// instead of two that can drift apart.
//
// `$` and `@` are the UTAGE star-shape modifiers (`1$,` forces a star-shaped
// TAP, `1$$,` a rotating one, `@` reverts a star to a normal TAP). The class
// admits them anywhere a modifier may sit; `parse_tap_or_touch_note` decides
// which combinations mean something.
static TAP_TOUCH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
            ^([A-Ea-e][1-8]|[1-8]|[1-8][1-8]|[CEce])  # Button or touch location (zones case-insensitive)
            ([xhfb$@]*)                          # Modifiers
            (\[[^\[\]]*\])?                      # Optional hold duration, validated separately
            ([xhfb$@]*)                          # Optional modifiers after duration
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
    let modifiers_after = caps.get(4).map(|m| m.as_str()).unwrap_or("").to_string();

    // A bracket that matched the regex but not the duration grammar is an
    // error, not an absent duration — otherwise `3h[oops]` would quietly become
    // a pseudo-hold.
    let hold_duration = match caps.get(3) {
        Some(bracket) => Some(parse_duration_bracket(bracket.as_str()).ok_or_else(|| {
            format!(
                "Invalid duration '{}' in note '{}'",
                bracket.as_str(),
                note_str
            )
        })?),
        None => None,
    };

    let Modifiers {
        is_break,
        is_ex,
        is_firework,
        is_hold,
        stars,
        plain,
    } = parse_note_modifiers(&modifiers, &modifiers_after);

    // `$` forces the star shape; `@` reverts it. Saying both on one note is a
    // contradiction, and three stars names nothing — rejected rather than
    // guessed at, on the same grounds as ADR-0012's duplicate markers.
    if stars > 2 {
        return Err(format!(
            "'$' may appear once (star TAP) or twice (rotating star TAP), not {} times, in '{}'",
            stars, note_str
        ));
    }
    if stars > 0 && plain {
        return Err(format!(
            "'$' (star-shaped TAP) and '@' (normal TAP) contradict each other in '{}'",
            note_str
        ));
    }

    // Two-digit shorthand EACH notation (e.g. "12" = buttons 1 and 2 simultaneously)
    if raw_loc.len() == 2 && raw_loc.chars().all(|c| c.is_ascii_digit()) {
        if stars > 0 {
            return Err(format!(
                "'$' is not defined on the two-digit EACH shorthand '{}'; write '{}$/{}$'",
                note_str,
                &raw_loc[0..1],
                &raw_loc[1..2]
            ));
        }
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
        if stars > 0 {
            // The notation defines `$` on a TAP only: "turns it into the
            // star-shaped TAP that normally only appears when a SLIDE is
            // placed". A star-shaped HOLD is not a thing.
            if is_hold {
                return Err(format!(
                    "'$' applies to a TAP, not a HOLD, in '{}'",
                    note_str
                ));
            }
            return Ok(vec![Note {
                is_break,
                is_firework,
                is_ex,
                offset_ms: 0,
                kind: NoteKind::SlideStar {
                    button: btn_num,
                    rotates: stars == 2,
                },
            }]);
        }
        return Ok(build_button_note(
            btn_num,
            is_hold,
            hold_duration,
            is_break,
            is_firework,
            is_ex,
        ));
    }

    // A TOUCH has no star shape to force.
    if stars > 0 {
        return Err(format!(
            "'$' applies to a TAP, not a TOUCH, in '{}'",
            note_str
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

/// Everything the two modifier slots said, before any of it is validated.
struct Modifiers {
    is_break: bool,
    is_ex: bool,
    is_firework: bool,
    is_hold: bool,
    /// How many `$` were written: 1 is a star-shaped TAP, 2 a rotating one.
    /// Counted rather than flagged so `$$$` can be rejected instead of
    /// collapsing into `$$`.
    stars: usize,
    /// `@` was written. On a slide head it reverts the star to a normal TAP;
    /// on a note that was never a star it is a no-op, and accepted as one.
    plain: bool,
}

fn parse_note_modifiers(modifiers: &str, modifiers_after: &str) -> Modifiers {
    let all = format!("{modifiers}{modifiers_after}");
    Modifiers {
        is_break: all.contains('b'),
        is_ex: all.contains('x'),
        is_firework: all.contains('f'),
        is_hold: all.contains('h'),
        stars: all.matches('$').count(),
        plain: all.contains('@'),
    }
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
            offset_ms: 0,
            kind: NoteKind::Tap(btn1),
        },
        Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind: NoteKind::Tap(btn2),
        },
    ]
}

/// A hold written without a bracket is a pseudo-hold: SEGA's fan book gives the
/// implied held-down length as a 1280th note, so `3h,` is `3h[1280:1],`.
const PSEUDO_HOLD: Duration = Duration::Simple {
    divider: 1280,
    count: 1,
};

fn build_button_note(
    btn_num: usize,
    is_hold: bool,
    hold_duration: Option<Duration>,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Vec<Note> {
    if is_hold {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind: NoteKind::TapHold {
                button: btn_num,
                duration: hold_duration.unwrap_or(PSEUDO_HOLD),
            },
        }]
    } else {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind: NoteKind::Tap(btn_num),
        }]
    }
}

fn build_touch_note(
    zone: char,
    index: usize,
    is_hold: bool,
    hold_duration: Option<Duration>,
    is_break: bool,
    is_firework: bool,
    is_ex: bool,
) -> Vec<Note> {
    if is_hold {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind: NoteKind::TouchHold {
                value: index,
                group: zone,
                duration: hold_duration.unwrap_or(PSEUDO_HOLD),
            },
        }]
    } else {
        vec![Note {
            is_break,
            is_firework,
            is_ex,
            offset_ms: 0,
            kind: NoteKind::Touch {
                value: index,
                group: zone,
            },
        }]
    }
}

#[cfg(test)]
mod tests {
    // `parse` and `parse_err` are imported for the stubs below, not for the
    // tests that already exist.
    #[allow(unused_imports)]
    use super::super::testutil::{parse, parse_err, parse_one, plain, simple};
    use crate::systems::component::{Duration, NoteKind};

    #[test]
    fn bare_digit_is_a_tap() {
        assert_eq!(parse_one("1"), plain(NoteKind::Tap(1)));
    }

    #[test]
    fn hold_carries_its_bracket_duration() {
        assert_eq!(
            parse_one("3h[4:1]"),
            plain(NoteKind::TapHold {
                button: 3,
                duration: simple(4, 1),
            })
        );
    }

    /// Every button 1-8 is a tap, and nothing outside that range is.
    #[test]
    fn all_eight_buttons_are_taps() {
        for btn in 1..=8 {
            assert_eq!(parse_one(&btn.to_string()), plain(NoteKind::Tap(btn)));
        }
    }

    // --- Note kinds ---------------------------------------------------------

    /// A pseudo-hold is `h` with no bracket: `3h,`. Per the notation doc it is
    /// treated as `[1280:1]` — SEGA's fan book gives the implied held-down
    /// length as a 1280th note.
    ///
    /// Asserted explicitly because it is exactly the kind of magic number that
    /// gets "cleaned up" into a round 1024 and silently changes note lengths.
    #[test]
    fn pseudo_hold_defaults_to_1280_1() {
        assert_eq!(
            parse_one("3h"),
            plain(NoteKind::TapHold {
                button: 3,
                duration: simple(1280, 1),
            })
        );
    }

    /// Touch notes name a sensor group A-E and an index. `group` is stored
    /// uppercased, so the lowercase spelling the regex also accepts must agree.
    #[test]
    fn touch_zones_normalise_case() {
        for (input, group, value) in [
            ("A1", 'A', 1),
            ("B5", 'B', 5),
            ("D7", 'D', 7),
            ("E3", 'E', 3),
            ("e3", 'E', 3),
            ("b5", 'B', 5),
        ] {
            assert_eq!(parse_one(input), plain(NoteKind::Touch { value, group }));
        }
    }

    /// Group C has two sensors, but no TOUCH ever appears at one individually —
    /// a centre touch is written bare, `C,`. `build_touch_note` defaults the
    /// index to 1 via `unwrap_or(1)`.
    ///
    /// Divergence from the notation doc: it says `C1,` and `C2,` both behave
    /// exactly as `C,`. Here `C2` keeps `value: 2`. Harmless as long as nothing
    /// downstream distinguishes the two — `systems/visual/` draws the centre
    /// either way — but it is a difference, so it is pinned here.
    #[test]
    fn bare_centre_touch_defaults_to_index_one() {
        let centre = plain(NoteKind::Touch {
            value: 1,
            group: 'C',
        });
        assert_eq!(parse_one("C"), centre);
        assert_eq!(parse_one("C1"), centre);
        assert_eq!(
            parse_one("C2"),
            plain(NoteKind::Touch {
                value: 2,
                group: 'C',
            })
        );
    }

    /// TOUCH HOLD is a HOLD with the button number replaced by a sensor. The
    /// pseudo-hold fallback applies here too (`Ch,` judges instantly), and `h`
    /// and `f` may appear in either order.
    #[test]
    fn touch_hold_with_and_without_duration() {
        assert_eq!(
            parse_one("C1h[4:3]"),
            plain(NoteKind::TouchHold {
                value: 1,
                group: 'C',
                duration: simple(4, 3),
            })
        );
        assert_eq!(
            parse_one("Ch"),
            plain(NoteKind::TouchHold {
                value: 1,
                group: 'C',
                duration: simple(1280, 1),
            })
        );

        // `Chf[1:2]` and `Cfh[1:2]` are the same note.
        let firework = parse_one("Chf[1:2]");
        assert_eq!(firework, parse_one("Cfh[1:2]"));
        assert!(firework.is_firework);
        assert_eq!(
            firework.kind,
            NoteKind::TouchHold {
                value: 1,
                group: 'C',
                duration: simple(1, 2),
            }
        );
    }

    // --- Modifiers ----------------------------------------------------------

    /// `b`=BREAK, `x`=EX, `f`=firework. `parse_note_modifiers` reads both the
    /// pre-bracket and post-bracket slots, so each flag must be settable from
    /// either side: `5hb[2:1]` and `5bh[2:1]` are the same note per the doc.
    #[test]
    fn modifiers_are_read_before_and_after_the_bracket() {
        assert!(parse_one("1b").is_break);
        assert!(parse_one("1h[4:1]b").is_break);
        assert!(parse_one("1x").is_ex);
        assert!(parse_one("1h[4:1]x").is_ex);
        assert!(parse_one("B7f").is_firework);
        assert!(parse_one("Ch[4:1]f").is_firework);

        // Order around `h` does not matter.
        assert_eq!(parse_one("5hb[2:1]"), parse_one("5bh[2:1]"));
    }

    /// `x`, `h` and `b` combine in any order — the doc's `7bxh[α:β]` is an
    /// EX BREAK HOLD.
    #[test]
    fn modifiers_combine() {
        let n = parse_one("1bx");
        assert!(n.is_break && n.is_ex && !n.is_firework);
        assert_eq!(n.kind, NoteKind::Tap(1));
        assert_eq!(parse_one("1xb"), n);

        let hold = parse_one("7bxh[4:1]");
        assert!(hold.is_break && hold.is_ex);
        assert_eq!(
            hold.kind,
            NoteKind::TapHold {
                button: 7,
                duration: simple(4, 1),
            }
        );
    }

    // --- EACH shorthand -----------------------------------------------------

    /// `12,` is two simultaneous taps, not button twelve — the one EACH that
    /// may omit the `/`. This is also the only place a non-slide returns more
    /// than one note.
    ///
    /// The regex earns its keep here: `TAP_TOUCH_RE`'s alternation lists
    /// `[1-8]` *before* `[1-8][1-8]`, and alternation is leftmost-first, not
    /// longest-match. `"12"` first matches just `"1"`, fails at `$` with `"2"`
    /// left over, and only parses because the engine backtracks into the
    /// two-digit branch. Reordering that alternation would break this silently.
    #[test]
    fn two_digit_shorthand_is_two_taps() {
        assert_eq!(
            parse("12"),
            vec![plain(NoteKind::Tap(1)), plain(NoteKind::Tap(2))]
        );
        // Order is positional, not sorted.
        assert_eq!(
            parse("21"),
            vec![plain(NoteKind::Tap(2)), plain(NoteKind::Tap(1))]
        );
    }

    /// Divergence: the doc says only an EACH made *entirely of non-BREAK TAPs*
    /// may drop the `/`, so `12b` is not legal notation. It is accepted here
    /// and marks both taps BREAK.
    ///
    /// Being lenient about input nobody should write is defensible; silently
    /// reinterpreting it is what this pins down. If a future change makes `12b`
    /// an error instead, that is a deliberate call and this test should be
    /// updated to match, not deleted.
    #[test]
    fn two_digit_shorthand_accepts_a_break_the_spec_forbids() {
        let notes = parse("12b");
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().all(|n| n.is_break));
    }

    // --- Rejection ----------------------------------------------------------

    /// `parse_note` returns `Err`, never panics. `build_button_note` and
    /// `build_touch_note` both lean on `unwrap_or`, so anything that reaches
    /// them with a surprising capture would fall back rather than fail loudly —
    /// these inputs confirm nothing reaches them at all.
    #[test]
    fn malformed_notes_return_err_never_panic() {
        for input in [
            "",      // empty
            "9",     // above the 1-8 range
            "0",     // below it
            "99",    // two-digit, both out of range
            "Z1",    // unknown sensor group
            "F1",    // group letter past E
            "1[",    // stray bracket
            "1[4:",  // unterminated bracket
            "b",     // lone modifier
            "h",     // lone hold marker
            "[4:1]", // duration with no note
        ] {
            parse_err(input);
        }
    }

    /// All three HOLD duration forms from the notation doc, not just `[N:M]`.
    ///
    /// `TAP_TOUCH_RE` captures the bracket loosely and hands it to
    /// `parse_duration_bracket`, so a HOLD understands exactly what a slide
    /// does — one duration grammar, not two that can drift apart.
    #[test]
    fn hold_accepts_every_duration_form() {
        assert_eq!(
            parse_one("5h[2:1]").kind,
            NoteKind::TapHold {
                button: 5,
                duration: simple(2, 1),
            }
        );
        assert_eq!(
            parse_one("4h[#5.678]").kind,
            NoteKind::TapHold {
                button: 4,
                duration: Duration::Seconds(5.678),
            }
        );
        assert_eq!(
            parse_one("4h[150#2:1]").kind,
            NoteKind::TapHold {
                button: 4,
                duration: Duration::BpmOverride {
                    bpm: 150.0,
                    divider: 2,
                    count: 1,
                },
            }
        );

        // Touch holds take the same forms.
        assert_eq!(
            parse_one("Ch[#2.5]").kind,
            NoteKind::TouchHold {
                value: 1,
                group: 'C',
                duration: Duration::Seconds(2.5),
            }
        );
    }

    /// A bracket that matches the note grammar but not the duration grammar is
    /// an error, not an absent duration — otherwise `3h[oops]` would quietly
    /// degrade into a pseudo-hold and judge instantly.
    #[test]
    fn unparseable_hold_bracket_is_an_error() {
        let err = parse_err("3h[a:b]");
        assert!(err.contains("Invalid duration"), "{err}");
        parse_err("3h[]"); // empty
        parse_err("3h[8]"); // divider with no count
        parse_err("3h[#]"); // bare hash
    }

    /// Quirk: `parse_note` picks tap-vs-slide by scanning the *whole* string for
    /// a shape character, bracket contents included. So `3h[oops]` routes to the
    /// slide parser on the `p` and fails there instead of as a bad duration.
    ///
    /// Harmless in practice — every legal duration bracket holds only digits,
    /// `#`, `:` and `.`, none of which is a shape character — so the misrouting
    /// only ever changes which error message malformed input gets. Recorded
    /// because the next person to widen the duration grammar needs to know the
    /// dispatch happens first.
    #[test]
    fn a_bracket_holding_a_shape_char_routes_to_the_slide_parser() {
        let err = parse_err("3h[oops]");
        assert!(err.contains("Slide shape"), "{err}");
    }

    /// The backtick is a *token*-level separator, not a note-level one, so
    /// `parse_note` never sees it — `parse_chart` splits on it first.
    ///
    /// Asserted here so the split cannot quietly move down a layer: if it ever
    /// did, a backtick would start parsing as part of a note and the sub-comma
    /// offset would be lost. See
    /// `chart::tests::backtick_delays_the_following_notes`.
    #[test]
    fn backtick_is_handled_above_parse_note() {
        parse_err("1`2");
    }

    // --- UTAGE star modifiers -----------------------------------------------

    /// `1$,` is the star-shaped TAP that normally only appears as a slide head,
    /// and `1$$,` is the same star rotating. Per the notation doc's "Change
    /// Normal TAP to Star-shaped TAP".
    ///
    /// The count is what distinguishes them, so it is asserted on both: a
    /// `contains('$')` check would silently flatten `$$` into `$`.
    #[test]
    fn dollar_makes_a_star_tap_and_two_make_it_rotate() {
        assert_eq!(
            parse_one("1$"),
            plain(NoteKind::SlideStar {
                button: 1,
                rotates: false,
            })
        );
        assert_eq!(
            parse_one("1$$"),
            plain(NoteKind::SlideStar {
                button: 1,
                rotates: true,
            })
        );
    }

    /// "This can be combined with BREAK or EX-TAP; when `$`, `b`, and `x` all
    /// appear together, order doesn't matter."
    #[test]
    fn star_tap_combines_with_break_and_ex_in_any_order() {
        let star = parse_one("1$bx");
        assert!(star.is_break && star.is_ex);
        assert_eq!(
            star.kind,
            NoteKind::SlideStar {
                button: 1,
                rotates: false,
            }
        );
        assert_eq!(parse_one("1b$x"), star);
        assert_eq!(parse_one("1xb$"), star);
        assert_eq!(parse_one("1$$b"), parse_one("1b$$"));
    }

    /// `@` reverts a star-shaped TAP to a normal one. On a bare TAP there is
    /// no star to revert, so it is a no-op: `1@,` is exactly `1,`. The slide
    /// case, where it does something, is `slide::tests::at_sign_makes_the_head_a_plain_tap`.
    #[test]
    fn at_sign_on_a_bare_tap_is_a_no_op() {
        assert_eq!(parse_one("1@"), plain(NoteKind::Tap(1)));
        assert_eq!(parse_one("1@b"), parse_one("1b"));
    }

    /// Combinations the notation does not define are rejected, not guessed at:
    /// three stars, a star that is also `@`, a star on a HOLD, a star on a
    /// TOUCH, and a star on the two-digit EACH shorthand (write `1$/2$`).
    #[test]
    fn undefined_star_combinations_are_errors() {
        assert!(parse_err("1$$$").contains("not 3 times"));
        assert!(parse_err("1$@").contains("contradict"));
        assert!(parse_err("1@$").contains("contradict"));
        assert!(parse_err("1h$[4:1]").contains("not a HOLD"));
        assert!(parse_err("1$h").contains("not a HOLD"));
        assert!(parse_err("C$").contains("not a TOUCH"));
        assert!(parse_err("B5$").contains("not a TOUCH"));
        assert!(parse_err("12$").contains("EACH shorthand"));
    }
}
