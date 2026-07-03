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
