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
        let current_str = strip_meta_tokens(token, &mut events);

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

// Strip all BPM changes, resolution changes, and absolute-length markers from
// one token, pushing the corresponding events. Returns the remaining string.
// Markers can appear in any order and position, so we loop until none remain.
// AbsoluteLength {#S} is checked before ResolutionChange {N} because {#0.35}
// would partially match the integer-only resolution pattern.
fn strip_meta_tokens(token: &str, events: &mut Vec<ChartEvent>) -> String {
    let mut current_str = token.to_string();

    loop {
        let mut found = false;

        if let Some(caps) = ABS_LEN_REGEX.captures(&current_str)
            && let Ok(seconds) = caps[1].parse::<f64>()
        {
            events.push(ChartEvent::AbsoluteLength(seconds));
            let m = caps.get(0).unwrap();
            current_str = format!("{}{}", &current_str[..m.start()], &current_str[m.end()..]);
            found = true;
        }

        if let Some(caps) = BPM_REGEX.captures(&current_str)
            && let Ok(bpm) = caps[1].parse::<f32>()
        {
            events.push(ChartEvent::BpmChange(bpm));
            let m = caps.get(0).unwrap();
            current_str = format!("{}{}", &current_str[..m.start()], &current_str[m.end()..]);
            found = true;
        }

        if let Some(caps) = RES_REGEX.captures(&current_str)
            && let Ok(resolution) = caps[1].parse::<u32>()
        {
            events.push(ChartEvent::ResolutionChange(resolution));
            let m = caps.get(0).unwrap();
            current_str = format!("{}{}", &current_str[..m.start()], &current_str[m.end()..]);
            found = true;
        }

        if !found {
            break;
        }
    }

    current_str
}
