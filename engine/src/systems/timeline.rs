//! The chart as the timeline draws it (visualizer-majdata 07): its length,
//! its measures and how many notes of each kind fall in each. Built from the
//! same timed events the engine plays, so the graph cannot disagree with
//! what is heard.

use super::component::{ChartEvent, NoteKind, TimedEvent};
use super::visual::schedule::note_end;

/// Beats since the chart began at chart time `t`. Each event's bpm holds
/// until the next event; before the first, the first's; after the last,
/// the last's. Simai has no time signature: a measure is four beats.
pub fn beat_at(events: &[TimedEvent], t: f64) -> f64 {
    let Some(first) = events.first() else {
        return 0.0;
    };
    let (mut beats, mut time, mut bpm) = (0.0, 0.0, first.bpm as f64);
    for event in events {
        if event.time >= t {
            break;
        }
        beats += (event.time - time) * bpm / 60.0;
        time = event.time;
        bpm = event.bpm as f64;
    }
    beats + (t - time) * bpm / 60.0
}

/// The chart time at which `beat` beats have passed: `beat_at`'s inverse.
pub fn time_at_beat(events: &[TimedEvent], beat: f64) -> f64 {
    let Some(first) = events.first() else {
        return 0.0;
    };
    let (mut beats, mut time, mut bpm) = (0.0, 0.0, first.bpm as f64);
    for event in events {
        let at_event = beats + (event.time - time) * bpm / 60.0;
        if at_event >= beat {
            break;
        }
        beats = at_event;
        time = event.time;
        bpm = event.bpm as f64;
    }
    time + (beat - beats) * 60.0 / bpm
}

/// One measure: its start (chart seconds) and its notes by timeline colour.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureCounts {
    pub start: f64,
    pub tap: u32,
    pub slide: u32,
    pub touch: u32,
    pub hold: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// When the last note finishes (chart seconds).
    pub length: f64,
    pub measures: Vec<MeasureCounts>,
}

impl Timeline {
    /// Hand-built like `FrameStats::to_json`: the engine has no serde.
    pub fn to_json(&self) -> String {
        let measures: Vec<String> = self
            .measures
            .iter()
            .map(|m| {
                format!(
                    r#"{{"start":{:.6},"tap":{},"slide":{},"touch":{},"hold":{}}}"#,
                    m.start, m.tap, m.slide, m.touch, m.hold
                )
            })
            .collect();
        format!(
            r#"{{"length":{:.6},"measures":[{}]}}"#,
            self.length,
            measures.join(",")
        )
    }
}

/// A note at a measure line belongs to the measure it starts.
const ON_THE_LINE: f64 = 1e-9;

/// When the chart's last note finishes (chart seconds); 0 for no notes.
pub fn chart_length(events: &[TimedEvent]) -> f64 {
    events
        .iter()
        .filter_map(|e| match &e.event {
            ChartEvent::NoteGroup(notes) => Some(
                notes
                    .iter()
                    .map(|n| note_end(&n.kind, e.time, e.bpm))
                    .fold(0.0, f64::max),
            ),
            _ => None,
        })
        .fold(0.0, f64::max)
}

/// The longest a note stays on screen after its hit (chart seconds).
pub fn longest_tail(events: &[TimedEvent]) -> f64 {
    events
        .iter()
        .filter_map(|e| match &e.event {
            ChartEvent::NoteGroup(notes) => Some(
                notes
                    .iter()
                    .map(|n| note_end(&n.kind, e.time, e.bpm) - e.time)
                    .fold(0.0, f64::max),
            ),
            _ => None,
        })
        .fold(0.0, f64::max)
}

pub fn build_timeline(events: &[TimedEvent]) -> Timeline {
    let length = chart_length(events);
    let mut measures = Vec::new();
    loop {
        let start = time_at_beat(events, 4.0 * measures.len() as f64);
        if start >= length {
            break;
        }
        measures.push(MeasureCounts {
            start,
            tap: 0,
            slide: 0,
            touch: 0,
            hold: 0,
        });
    }

    for event in events {
        let ChartEvent::NoteGroup(notes) = &event.event else {
            continue;
        };
        let index = ((beat_at(events, event.time) + ON_THE_LINE) / 4.0).floor() as usize;
        let Some(measure) = measures.get_mut(index) else {
            continue;
        };
        for note in notes {
            // The timeline's colours: a slide counts by its star tap, so one
            // with no star (headless) counts nothing.
            match note.kind {
                NoteKind::Tap(_) => measure.tap += 1,
                NoteKind::TapHold { .. } => measure.hold += 1,
                NoteKind::Touch { .. } | NoteKind::TouchHold { .. } => measure.touch += 1,
                NoteKind::Slide { .. } | NoteKind::SlideStar { .. } => measure.slide += 1,
                NoteKind::HeadlessSlide { .. } => {}
            }
        }
    }

    Timeline { length, measures }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::chart_playback::ChartPlayback;

    fn events(chart: &str) -> Vec<TimedEvent> {
        let mut playback = ChartPlayback::default();
        playback.load_chart(simai::parse_chart(chart).unwrap());
        playback.timed_events().to_vec()
    }

    #[test]
    fn beats_follow_bpm_changes() {
        let events = events("(120){4}1,1,(240)1,1,E");
        assert!((beat_at(&events, 1.0) - 2.0).abs() < 1e-9);
        assert!((beat_at(&events, 2.0) - 6.0).abs() < 1e-9);
        for x in [0.0, 0.3, 1.0, 1.7, 3.9] {
            assert!((time_at_beat(&events, beat_at(&events, x)) - x).abs() < 1e-9);
        }
    }

    /// Review Focus 4.
    #[test]
    fn four_beats_make_a_measure() {
        let tl = build_timeline(&events("(120){4}1,1,1,1,1,E"));
        let starts: Vec<f64> = tl.measures.iter().map(|m| m.start).collect();
        assert_eq!(starts, vec![0.0, 2.0]);
        assert_eq!(tl.measures[0].tap, 4);
        assert_eq!(tl.measures[1].tap, 1);
    }

    #[test]
    fn notes_count_by_kind() {
        let tl = build_timeline(&events("(120){4}1/2h[4:1]/3-5[8:1]/B1,Ch[4:1],1$,E"));
        let m = &tl.measures[0];
        assert_eq!((m.tap, m.hold, m.slide, m.touch), (1, 1, 2, 2));
    }

    #[test]
    fn the_chart_ends_when_its_last_note_does() {
        let tl = build_timeline(&events("(120){4}1h[1:1],E"));
        assert!((tl.length - 2.1).abs() < 1e-6);
    }

    #[test]
    fn an_empty_chart_has_no_measures() {
        let tl = build_timeline(&[]);
        assert_eq!(tl.length, 0.0);
        assert!(tl.measures.is_empty());
    }

    #[test]
    fn json_shape() {
        let tl = Timeline {
            length: 4.25,
            measures: vec![
                MeasureCounts {
                    start: 0.0,
                    tap: 4,
                    slide: 0,
                    touch: 0,
                    hold: 0,
                },
                MeasureCounts {
                    start: 2.0,
                    tap: 1,
                    slide: 2,
                    touch: 3,
                    hold: 1,
                },
            ],
        };
        assert_eq!(
            tl.to_json(),
            r#"{"length":4.250000,"measures":[{"start":0.000000,"tap":4,"slide":0,"touch":0,"hold":0},{"start":2.000000,"tap":1,"slide":2,"touch":3,"hold":1}]}"#
        );
    }
}
