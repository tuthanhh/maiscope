//! Where a note is at a given chart time (visualizer-majdata 05).
//!
//! Every note runs the same course: it grows in, travels to the ring, and
//! is hit at its event time; then, by kind, it holds, waits and traces, or
//! pops. Each phase has a length in chart seconds, so the phase a note is in
//! — and how far through — is a function of the chart clock alone. Pause,
//! speed changes and seeking follow from that with no per-note bookkeeping.

// Until update_movement reads it (visualizer-majdata 06).
#![allow(dead_code)]

use crate::systems::component::NoteKind;
use crate::systems::visual::slide_path::slide_span_secs;
use crate::systems::{GROWING, MOVING};

/// A note's phases, in the order it goes through them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Phase {
    Growing,
    Moving,
    Holding,
    Waiting,
    Sliding,
    Dying,
}

/// The phase a note is in, and how far through it, in chart seconds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PhaseAt {
    pub phase: Phase,
    pub elapsed: f32,
    pub duration: f32,
}

/// A tap's or a slide star's pop after the hit.
const TAP_DYING: f64 = 0.25;
/// A touch's flash on the hit, then its burst.
const TOUCH_HOLDING: f64 = 0.05;
const TOUCH_DYING: f64 = 0.25;
/// A hold's pop at its release.
const HOLD_DYING: f64 = 0.1;

/// The phases after the hit, with their lengths. Two at most; a padding
/// entry has length zero and is never current.
fn after_hit(kind: &NoteKind, bpm: f32) -> [(Phase, f64); 2] {
    match kind {
        NoteKind::Tap(_) | NoteKind::SlideStar { .. } => {
            [(Phase::Dying, TAP_DYING), (Phase::Dying, 0.0)]
        }
        NoteKind::Touch { .. } => [(Phase::Holding, TOUCH_HOLDING), (Phase::Dying, TOUCH_DYING)],
        NoteKind::TapHold { duration, .. } | NoteKind::TouchHold { duration, .. } => [
            (Phase::Holding, duration.hold_secs(bpm) as f64),
            (Phase::Dying, HOLD_DYING),
        ],
        NoteKind::Slide {
            segments,
            shared_duration,
            ..
        }
        | NoteKind::HeadlessSlide {
            segments,
            shared_duration,
            ..
        } => {
            let (wait, total) = slide_span_secs(segments, *shared_duration, bpm);
            [
                (Phase::Waiting, wait as f64),
                (Phase::Sliding, total as f64),
            ]
        }
    }
}

/// The phase of a note hit at `hit` (chart seconds) at chart time `t`;
/// `None` before it spawns and after it ends. The approach lasts
/// `(GROWING + MOVING) / note_speed` chart seconds whatever the play speed
/// (see `ChartPlayback::approach_time`).
pub fn phase_at(kind: &NoteKind, hit: f64, bpm: f32, note_speed: f32, t: f64) -> Option<PhaseAt> {
    let speed = note_speed as f64;
    let [first, second] = after_hit(kind, bpm);
    let spans = [
        (Phase::Growing, GROWING / speed),
        (Phase::Moving, MOVING / speed),
        first,
        second,
    ];
    let mut start = hit - (GROWING + MOVING) / speed;
    for (phase, length) in spans {
        let end = start + length;
        if t >= start && t < end {
            return Some(PhaseAt {
                phase,
                elapsed: (t - start) as f32,
                duration: length as f32,
            });
        }
        start = end;
    }
    None
}

/// The chart time a note hit at `hit` finishes its last phase.
pub fn note_end(kind: &NoteKind, hit: f64, bpm: f32) -> f64 {
    hit + after_hit(kind, bpm)
        .iter()
        .map(|(_, length)| length)
        .sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::component::{ChartEvent, NoteKind};
    use crate::systems::visual::resources::ButtonLayout;
    use crate::systems::visual::slide_path::{build_slide_trace, trace_total_secs};

    fn first_kind(chart: &str) -> NoteKind {
        simai::parse_chart(chart)
            .unwrap()
            .into_iter()
            .find_map(|event| match event {
                ChartEvent::NoteGroup(notes) => Some(notes[0].kind.clone()),
                _ => None,
            })
            .expect("a note in the chart")
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn a_tap_is_growing_then_moving_then_dying() {
        let tap = first_kind("(120){4}1,E");
        let at = |t| phase_at(&tap, 2.0, 120.0, 7.0, t);
        assert!(at(2.0 - 3.5 / 7.0 - 0.01).is_none());
        assert_eq!(at(2.0 - 2.0 / 7.0 - 0.01).unwrap().phase, Phase::Growing);
        let moving = at(2.0 - 0.1).unwrap();
        assert_eq!(moving.phase, Phase::Moving);
        assert!(close(moving.duration, 2.0 / 7.0));
        let dying = at(2.1).unwrap();
        assert_eq!(dying.phase, Phase::Dying);
        assert!(close(dying.elapsed, 0.1));
        assert!(at(2.26).is_none());
    }

    #[test]
    fn a_hold_holds_for_its_musical_length() {
        let hold = first_kind("(120){4}1h[1:1],E");
        let holding = phase_at(&hold, 0.0, 120.0, 7.0, 1.0).unwrap();
        assert_eq!(holding.phase, Phase::Holding);
        assert!(close(holding.duration, 2.0));
        assert!(close(holding.elapsed, 1.0));
        assert!((note_end(&hold, 0.0, 120.0) - 2.1).abs() < 1e-4);
    }

    #[test]
    fn a_slide_waits_then_traces() {
        let slide = first_kind("(120){4}1-5[8:1],E");
        let NoteKind::Slide {
            segments,
            shared_duration,
            ..
        } = &slide
        else {
            panic!("a slide");
        };
        let (wait, total) = slide_span_secs(segments, *shared_duration, 120.0);
        let hit = 1.0;
        let at = |t| phase_at(&slide, hit, 120.0, 7.0, t);
        assert_eq!(at(hit + wait as f64 / 2.0).unwrap().phase, Phase::Waiting);
        let sliding = at(hit + (wait + total / 2.0) as f64).unwrap();
        assert_eq!(sliding.phase, Phase::Sliding);
        assert!(close(sliding.elapsed, total / 2.0));
        assert!(at(note_end(&slide, hit, 120.0) + 0.01).is_none());
    }

    #[test]
    fn slide_span_matches_the_built_path() {
        let layout = ButtonLayout::default();
        for chart in [
            "(120){4}1-5[8:1],E",
            "(120){4}1-3[4:1],E",
            "(120){4}1>4-7[8:3],E",
            "(120){4}1-4[8:1]-7[8:1],E",
            "(120){4}1-5[0.5##1.5],E",
        ] {
            let NoteKind::Slide {
                head_button,
                segments,
                shared_duration,
                ..
            } = first_kind(chart)
            else {
                panic!("{chart}: a slide");
            };
            let path = build_slide_trace(&segments, head_button, 120.0, shared_duration, &layout);
            let (wait, total) = slide_span_secs(&segments, shared_duration, 120.0);
            assert!(close(wait, path.wait_secs), "{chart}: wait");
            assert!(close(total, trace_total_secs(&path)), "{chart}: total");
        }
    }
}
