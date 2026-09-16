//! Duration-bracket parsing for hold and slide notes.
//!
//! Turns a `[...]` simai duration bracket into a [`Duration`].

use crate::systems::component::Duration;

/// Parse a duration bracket string into a `Duration`.
///
/// Supported formats (the outer `[` and `]` are included in `bracket_str`):
///
/// - `[N:M]`           → Simple { divider: N, count: M }
/// - `[BPM#N:M]`       → BpmOverride { bpm, divider: N, count: M }
/// - `[BPM#S]`          → BpmOverrideSeconds { bpm, seconds: S }
/// - `[W##S]`           → ExplicitWaitAndTrace { wait: W, trace: S }
/// - `[W##N:M]`         → ExplicitWaitBeats { wait: W, divider: N, count: M }
/// - `[W##BPM#N:M]`     → ExplicitWaitBpmBeats { wait: W, bpm, divider: N, count: M }
pub(super) fn parse_duration_bracket(bracket_str: &str) -> Option<Duration> {
    // Strip the surrounding '[' and ']'
    let inner = bracket_str.strip_prefix('[')?.strip_suffix(']')?;

    if inner.is_empty() {
        return None;
    }

    // --- Format with ## (explicit wait time in seconds) ---
    if let Some(pos) = inner.find("##") {
        let wait_part = &inner[..pos];
        let rest = &inner[pos + 2..]; // after "##"

        let wait_seconds = wait_part.parse::<f32>().ok()?;

        // rest can be:
        //   "1.5"         → trace in seconds         (ExplicitWaitAndTrace)
        //   "8:3"         → trace in beats at current BPM (ExplicitWaitBeats)
        //   "160#8:3"     → trace in beats at given BPM   (ExplicitWaitBpmBeats)
        if let Some(hash_pos) = rest.find('#') {
            // "BPM#N:M"
            let bpm_part = &rest[..hash_pos];
            let beat_part = &rest[hash_pos + 1..];
            let bpm = bpm_part.parse::<f32>().ok()?;
            let (divider, count) = parse_beat_spec(beat_part)?;
            Some(Duration::ExplicitWaitBpmBeats {
                wait_seconds,
                bpm,
                divider,
                count,
            })
        } else if let Some((divider, count)) = parse_beat_spec(rest) {
            // "N:M"
            Some(Duration::ExplicitWaitBeats {
                wait_seconds,
                divider,
                count,
            })
        } else {
            // Absolute seconds for trace
            let trace_seconds = rest.parse::<f32>().ok()?;
            Some(Duration::ExplicitWaitAndTrace {
                wait_seconds,
                trace_seconds,
            })
        }
    }
    // --- Format with single # (BPM override, wait = 1 beat at that BPM) ---
    else if let Some(hash_pos) = inner.find('#') {
        let bpm_part = &inner[..hash_pos];
        let rest = &inner[hash_pos + 1..]; // after "#"

        let bpm = bpm_part.parse::<f32>().ok()?;

        // rest can be:
        //   "8:3"  → BpmOverride { bpm, divider: 8, count: 3 }
        //   "2"    → BpmOverrideSeconds { bpm, seconds: 2.0 }
        if let Some((divider, count)) = parse_beat_spec(rest) {
            Some(Duration::BpmOverride {
                bpm,
                divider,
                count,
            })
        } else {
            let seconds = rest.parse::<f32>().ok()?;
            Some(Duration::BpmOverrideSeconds { bpm, seconds })
        }
    }
    // --- Simple format [N:M] ---
    else if let Some((divider, count)) = parse_beat_spec(inner) {
        Some(Duration::Simple { divider, count })
    } else {
        None
    }
}

/// Parse a beat specification like "8:3" into (divider=8, count=3).
/// Returns None if the string is not in "N:M" format.
fn parse_beat_spec(s: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() == 2 {
        let divider = parts[0].parse::<usize>().ok()?;
        let count = parts[1].parse::<usize>().ok()?;
        Some((divider, count))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a bracket that is expected to be valid.
    fn ok(bracket: &str) -> Duration {
        parse_duration_bracket(bracket)
            .unwrap_or_else(|| panic!("expected {bracket:?} to parse, got None"))
    }

    #[test]
    fn simple_beat_spec() {
        assert_eq!(
            ok("[8:1]"),
            Duration::Simple {
                divider: 8,
                count: 1
            }
        );
    }

    // --- One test per bracket format ---------------------------------------
    // Inputs match the worked examples in the SLIDE section of
    // `docs/reference/simai-notation.md`, so these check the mapping against
    // the spec rather than against the implementation's own idea of itself.

    /// `[160#8:3]` — wait one beat at 160 BPM, trace three 8th notes at 160.
    #[test]
    fn bpm_override_with_beats() {
        assert_eq!(
            ok("[160#8:3]"),
            Duration::BpmOverride {
                bpm: 160.0,
                divider: 8,
                count: 3
            }
        )
    }

    /// `[160#2]` — single `#` with no colon after it, so the tail is seconds:
    /// wait one beat at 160 BPM, trace for 2 seconds.
    ///
    /// The branch is selected purely by `parse_beat_spec` returning `None`, so
    /// this is the case that breaks if that helper ever gets more permissive.
    #[test]
    fn bpm_override_with_seconds() {
        assert_eq!(
            ok("[160#2]"),
            Duration::BpmOverrideSeconds {
                bpm: 160.0,
                seconds: 2.0
            }
        )
    }

    /// `[1.5##2.0]` — `##` then a bare float: wait, then trace, both seconds.
    #[test]
    fn explicit_wait_and_trace() {
        assert_eq!(
            ok("[1.5##2.0]"),
            Duration::ExplicitWaitAndTrace {
                wait_seconds: 1.5,
                trace_seconds: 2.0
            }
        )
    }

    /// `[1.5##8:3]` — `##` then a beat spec: trace in beats at the current BPM.
    #[test]
    fn explicit_wait_with_beats() {
        assert_eq!(
            ok("[1.5##8:3]"),
            Duration::ExplicitWaitBeats {
                wait_seconds: 1.5,
                divider: 8,
                count: 3
            }
        )
    }

    /// `[1.5##160#8:3]` — `##` then BPM then beats. The three-way nesting is
    /// the easiest to mis-slice; assert all four fields.
    #[test]
    fn explicit_wait_with_bpm_beats() {
        assert_eq!(
            ok("[1.5##160#8:3]"),
            Duration::ExplicitWaitBpmBeats {
                wait_seconds: 1.5,
                bpm: 160.0,
                divider: 8,
                count: 3
            }
        )
    }

    /// Decimals are allowed wherever a BPM or a wait appears — the notation doc
    /// calls for them explicitly, since BPM has to be exact.
    #[test]
    fn decimal_bpm_and_wait_are_accepted() {
        assert_eq!(
            ok("[174.5#8:3]"),
            Duration::BpmOverride {
                bpm: 174.5,
                divider: 8,
                count: 3,
            }
        );
        assert_eq!(
            ok("[1.234##2.5]"),
            Duration::ExplicitWaitAndTrace {
                wait_seconds: 1.234,
                trace_seconds: 2.5,
            }
        );
    }

    // --- Rejection ----------------------------------------------------------

    /// Every one of these returns `None`, never panics. `parse_duration_bracket`
    /// is all `?` on `Option`, so a panic here would be a real bug.
    #[test]
    fn malformed_brackets_return_none() {
        for input in [
            "8:1",     // no brackets at all
            "[]",      // empty
            "[8:1",    // unclosed
            "8:1]",    // unopened
            "[a:b]",   // non-numeric
            "[8]",     // divider with no count
            "[-8:1]",  // negative divider
            "[8:]",    // count missing
            "[:1]",    // divider missing
            "[#]",     // bare hash
            "[##]",    // bare double hash
            "[8:1:2]", // one colon too many
        ] {
            assert_eq!(
                parse_duration_bracket(input),
                None,
                "expected {input:?} to be rejected"
            );
        }
    }

    /// BUG: an absolute held-down length in seconds is unsupported.
    ///
    /// `docs/reference/simai-notation.md` (HOLD) specifies `4h[#5.678],` — hold
    /// for exactly 5.678 seconds. Here the `#` sits at index 0, so `bpm_part` is
    /// the empty string, `"".parse::<f32>()` fails, and the bracket is rejected.
    /// `[150#2:1]` from the same table works, because it has a BPM before the `#`.
    ///
    /// This bracket never reaches here from a HOLD anyway: `TAP_TOUCH_RE` admits
    /// only `\[(\d+):(\d+)\]`, so `4h[#5.678]` fails in `note.rs` first. See
    /// `note::tests::hold_rejects_non_simple_durations`.
    #[test]
    fn absolute_seconds_bracket_is_unsupported() {
        assert_eq!(parse_duration_bracket("[#5.678]"), None);
    }
}
