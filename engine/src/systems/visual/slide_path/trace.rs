//! Slide trace assembly: the star's path through a slide's MajdataPlay
//! arrows, the time/distance breakpoints that drive it, and its position
//! over time.

use bevy::prelude::*;

use super::super::RADIUS;
use super::super::component::{FanLanes, SlidePath};
use super::super::majdata::{SlideLayouts, place, prefab_for, shape_end};
use super::super::resources::ButtonLayout;
use crate::systems::component::{Duration, SlideSegment, SlideShape};

/// Per-segment `(wait_secs, trace_secs)` for a slide segment's duration.
///
/// When the duration carries no explicit wait/trace (`Simple` / `BpmOverride`
/// / `BpmOverrideSeconds`), the wait defaults to one beat at the *current
/// note's* BPM and the trace uses the same formula as hold notes. The
/// `ExplicitWait*` variants supply the wait (and, for `ExplicitWaitAndTrace`,
/// the trace) directly.
fn slide_timing(duration: Duration, note_bpm: f32) -> (f32, f32) {
    let beat = 60.0 / note_bpm;
    match duration {
        Duration::ExplicitWaitAndTrace {
            wait_seconds,
            trace_seconds,
        } => (wait_seconds, trace_seconds),
        Duration::ExplicitWaitBeats {
            wait_seconds,
            divider,
            count,
        } => (
            wait_seconds,
            count as f32 / divider as f32 * (240.0 / note_bpm),
        ),
        Duration::ExplicitWaitBpmBeats {
            wait_seconds,
            bpm,
            divider,
            count,
        } => (wait_seconds, count as f32 / divider as f32 * (240.0 / bpm)),
        Duration::Simple { divider, count } => {
            (beat, count as f32 / divider as f32 * (240.0 / note_bpm))
        }
        Duration::BpmOverride {
            bpm,
            divider,
            count,
        } => (beat, count as f32 / divider as f32 * (240.0 / bpm)),
        Duration::BpmOverrideSeconds { seconds, .. } => (beat, seconds),
        Duration::Seconds(seconds) => (beat, seconds),
    }
}

/// `(wait, trace_total)` of a slide in seconds, without its geometry: the
/// wait before the star moves (the first segment's), and how long it takes
/// to trace the whole path — the one shared duration, or the sum of each
/// segment's own. Tested against `build_slide_trace`'s breakpoints, so a
/// chart's timeline and the drawn slide agree.
pub fn slide_span_secs(
    segments: &[SlideSegment],
    shared_duration: bool,
    note_bpm: f32,
) -> (f32, f32) {
    let first = segments.first().map(|s| slide_timing(s.duration, note_bpm));
    let wait = first.map(|f| f.0).unwrap_or(0.0);
    let total = if shared_duration {
        first.map(|f| f.1).unwrap_or(0.0)
    } else {
        segments
            .iter()
            .map(|s| slide_timing(s.duration, note_bpm).1)
            .sum()
    };
    (wait, total)
}

/// One arrow of a slide as it is drawn: position (pixels), rotation
/// (radians, MajdataPlay's sprite convention: the arrow art points along -x),
/// distance along the star's path, and its row in the prefab (a fan's rows
/// each have their own image).
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedArrow {
    pub pos: Vec2,
    pub angle: f32,
    pub distance: f32,
    pub row: usize,
}

/// A slide ready to spawn: the star's path, its arrows, and for a fan the
/// three lanes its stars take.
pub struct BuiltSlide {
    pub path: SlidePath,
    pub arrows: Vec<PlacedArrow>,
    pub fan: Option<FanLanes>,
}

/// Build a slide from MajdataPlay's layouts (visualizer-majdata 14). The star
/// walks from the start key through every arrow in draw order, as
/// MajdataPlay's does; each segment of a chain starts at the key the previous
/// one ended on. A segment MajdataPlay rejects (`1-2`) has no arrows and its
/// star goes straight to the end key.
///
/// * `shared_duration` — when true, the (single) duration covers the whole
///   chained path and is split across segments by length (constant speed);
///   otherwise each segment is traced over its own duration, sequentially.
pub fn build_slide_from_layout(
    segments: &[SlideSegment],
    start_button: usize,
    note_bpm: f32,
    shared_duration: bool,
    layouts: &SlideLayouts,
    layout: &ButtonLayout,
) -> BuiltSlide {
    let key = |k: usize| layout.tap[k - 1] * RADIUS;
    let mut waypoints = vec![key(start_button)];
    let mut arrows = Vec::new();
    let mut seg_lengths: Vec<f32> = Vec::with_capacity(segments.len());
    let mut fan = None;
    let mut from = start_button;
    let mut distance = 0.0;

    let walk_to = |pos: Vec2, waypoints: &mut Vec<Vec2>, distance: &mut f32| {
        *distance += waypoints.last().map_or(0.0, |w| w.distance(pos));
        waypoints.push(pos);
    };

    for seg in segments {
        let before = distance;
        let drawn = prefab_for(&seg.shape, from)
            .and_then(|p| layouts.get(p.prefab).map(|bars| (p.mirror, bars)));
        match drawn {
            Some((mirror, bars)) => {
                for (row, bar) in bars.iter().enumerate() {
                    let (pos, angle) = place(bar, from, mirror);
                    walk_to(pos, &mut waypoints, &mut distance);
                    arrows.push(PlacedArrow {
                        pos,
                        angle,
                        distance,
                        row,
                    });
                }
            }
            None => {
                debug!("no MajdataPlay layout for {:?} from {from}", seg.shape);
                walk_to(key(shape_end(&seg.shape)), &mut waypoints, &mut distance);
            }
        }
        if let SlideShape::FanShape { ends: (a, b, c) } = seg.shape {
            let lanes: Vec<Vec<Vec2>> =
                [a, b, c].iter().map(|&e| vec![key(from), key(e)]).collect();
            let lengths = lanes.iter().map(|l| l[0].distance(l[1])).collect();
            fan = Some(FanLanes { lanes, lengths });
        }
        seg_lengths.push(distance - before);
        from = shape_end(&seg.shape);
    }

    let total_length: f32 = seg_lengths.iter().sum();
    let (_, shared_total) = slide_span_secs(segments, true, note_bpm);
    let mut breakpoints = Vec::with_capacity(segments.len());
    let mut cum_t = 0.0;
    let mut cum_d = 0.0;
    for (i, seg) in segments.iter().enumerate() {
        let secs_i = if shared_duration {
            if total_length > 0.0 {
                shared_total * seg_lengths[i] / total_length
            } else {
                shared_total / segments.len().max(1) as f32
            }
        } else {
            slide_timing(seg.duration, note_bpm).1
        };
        cum_t += secs_i;
        cum_d += seg_lengths[i];
        breakpoints.push((cum_t, cum_d));
    }

    BuiltSlide {
        path: SlidePath {
            waypoints,
            total_length,
            breakpoints,
        },
        arrows,
        fan,
    }
}

/// How far along its path (pixels) the star is, `frac` of the way through
/// the Sliding phase: each segment of a chain takes its own time
/// (`breakpoints`), so the star's speed changes at a joint when the
/// segments' durations differ.
pub fn star_distance(path: &SlidePath, frac: f32) -> f32 {
    let elapsed = frac * trace_total_secs(path);
    let mut prev_t = 0.0;
    let mut prev_d = 0.0;
    for &(t_end, d_end) in &path.breakpoints {
        if elapsed <= t_end {
            let span = t_end - prev_t;
            let local = if span > 0.0 {
                (elapsed - prev_t) / span
            } else {
                1.0
            };
            return prev_d + (d_end - prev_d) * local;
        }
        prev_t = t_end;
        prev_d = d_end;
    }
    path.total_length
}

/// Total trace time (seconds) of the Sliding phase.
pub fn trace_total_secs(path: &SlidePath) -> f32 {
    path.breakpoints.last().map(|b| b.0).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::component::NoteKind;
    use crate::systems::visual::majdata::SlideLayouts;
    use crate::systems::visual::{NOTE_RADIUS, RADIUS};

    fn slide(chart: &str) -> (usize, Vec<SlideSegment>, bool) {
        let events = simai::parse_chart(chart).unwrap();
        for event in events {
            if let simai::ChartEvent::NoteGroup(notes) = event
                && let NoteKind::Slide {
                    head_button,
                    segments,
                    shared_duration,
                    ..
                } = &notes[0].kind
            {
                return (*head_button, segments.clone(), *shared_duration);
            }
        }
        panic!("no slide in {chart}");
    }

    fn built(chart: &str) -> BuiltSlide {
        let (start, segments, shared) = slide(chart);
        build_slide_from_layout(
            &segments,
            start,
            120.0,
            shared,
            &SlideLayouts::load(),
            &ButtonLayout::default(),
        )
    }

    fn key_pos(k: usize) -> Vec2 {
        ButtonLayout::default().tap[k - 1] * RADIUS
    }

    /// Review Focus 2: a chain runs through the key where its segments meet.
    #[test]
    fn a_chained_slide_is_continuous() {
        for chart in ["(120){4}1-4[8:1]-7[8:1],E", "(120){4}1>4-7[8:3],E"] {
            let slide = built(chart);
            assert_eq!(slide.path.breakpoints.len(), 2, "{chart}");
            let joint = key_pos(4);
            let near = slide
                .path
                .waypoints
                .iter()
                .filter(|w| w.distance(joint) < 2.5 * NOTE_RADIUS)
                .count();
            assert!(near >= 2, "{chart}: the path passes key 4");
            for pair in slide.path.waypoints.windows(2) {
                assert!(
                    pair[0].distance(pair[1]) < 2.5 * NOTE_RADIUS,
                    "{chart}: a jump"
                );
            }
        }
    }

    /// Review Focus 2: arrows clear in the order the star reaches them.
    #[test]
    fn arrows_clear_in_order() {
        for chart in [
            "(120){4}1-5[8:1],E",
            "(120){4}1-4[8:1]-7[8:1],E",
            "(120){4}3q7[8:1],E",
        ] {
            let slide = built(chart);
            assert!(!slide.arrows.is_empty());
            for pair in slide.arrows.windows(2) {
                assert!(pair[0].distance <= pair[1].distance, "{chart}");
            }
        }
    }

    /// Review Focus 3.
    #[test]
    fn a_fan_has_eleven_rows_and_three_lanes() {
        for start in [1usize, 4, 7] {
            let end = (start + 3) % 8 + 1;
            let slide = built(&format!("(120){{4}}{start}w{end}[8:1],E"));
            assert_eq!(slide.arrows.len(), 11, "from {start}");
            let fan = slide.fan.expect("fan lanes");
            let ends: Vec<usize> = vec![end, end % 8 + 1, (end + 6) % 8 + 1];
            for lane in &fan.lanes {
                let last = *lane.last().unwrap();
                assert!(
                    ends.iter()
                        .any(|&k| last.distance(key_pos(k)) < 0.6 * NOTE_RADIUS),
                    "from {start}: lane ends at {last:?}"
                );
            }
        }
    }

    /// Decision 6: a segment MajdataPlay rejects draws no arrows and its star
    /// goes straight to the end key.
    #[test]
    fn a_rejected_segment_goes_straight() {
        let slide = built("(120){4}1-2[8:1],E");
        assert!(slide.arrows.is_empty());
        assert_eq!(slide.path.waypoints.last().copied(), Some(key_pos(2)));
    }

    /// Review Focus 2: each segment of a chain takes its own time. At 120
    /// BPM `[8:1]` is 0.25 s and `[4:1]` 0.5 s, so a third of the way
    /// through, the star is at the joint.
    #[test]
    fn each_segment_of_a_chain_takes_its_own_time() {
        let slide = built("(120){4}1-4[8:1]-7[4:1],E");
        let (_, joint) = slide.path.breakpoints[0];
        let at = star_distance(&slide.path, 1.0 / 3.0);
        assert!((at - joint).abs() < 1.0, "{at} vs {joint}");
    }
}
