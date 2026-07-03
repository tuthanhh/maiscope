//! Slide trace assembly: concatenate per-segment geometry, attach the
//! time/distance breakpoints that drive the tracing star, and sample the
//! star's position over time.

use bevy::prelude::Vec2;

use super::super::component::SlidePath;
use super::super::resources::ButtonLayout;
use super::generators::generate_points;
use super::geometry::{append_path_dedup, calculate_total_length, get_transform_at_distance};
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
    }
}

/// The end button a slide segment terminates on (used to chain segments).
fn slide_shape_end(shape: &SlideShape) -> usize {
    match shape {
        SlideShape::Straight { end }
        | SlideShape::ShortArc { end }
        | SlideShape::ClockwiseArc { end }
        | SlideShape::CounterClockwiseArc { end }
        | SlideShape::VShape { end }
        | SlideShape::PShape { end }
        | SlideShape::QShape { end }
        | SlideShape::GrandPShape { end }
        | SlideShape::GrandQShape { end }
        | SlideShape::GrandVShape { end, .. }
        | SlideShape::Thunderbolt { end, .. } => *end,
        SlideShape::FanShape { ends: (e1, ..) } => *e1,
    }
}

/// Build the full slide trace: concatenated waypoints plus the time/distance
/// breakpoints that drive the tracing star.
///
/// * `shared_duration` — when true, the (single) duration covers the whole
///   chained path and is split across segments by arc length (constant speed);
///   otherwise each segment is traced over its own duration, sequentially.
pub fn build_slide_trace(
    segments: &[SlideSegment],
    start_button: usize,
    note_bpm: f32,
    shared_duration: bool,
    layout: &ButtonLayout,
) -> SlidePath {
    let mut waypoints: Vec<Vec2> = Vec::new();
    let mut seg_lengths: Vec<f32> = Vec::with_capacity(segments.len());
    let mut current_start = start_button;

    for seg in segments {
        let pts = generate_points(&seg.shape, current_start, layout);
        seg_lengths.push(calculate_total_length(&pts));
        append_path_dedup(&mut waypoints, pts);
        current_start = slide_shape_end(&seg.shape);
    }

    let total_length: f32 = seg_lengths.iter().sum();
    let wait_secs = segments
        .first()
        .map(|s| slide_timing(s.duration, note_bpm).0)
        .unwrap_or(0.0);
    // Shared total = the single duration covering the whole path.
    let shared_total = segments
        .first()
        .map(|s| slide_timing(s.duration, note_bpm).1)
        .unwrap_or(0.0);

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

    SlidePath {
        waypoints,
        total_length,
        breakpoints,
        wait_secs,
    }
}

/// Total trace time (seconds) of the Sliding phase.
pub fn trace_total_secs(path: &SlidePath) -> f32 {
    path.breakpoints.last().map(|b| b.0).unwrap_or(0.0)
}

/// Position and facing angle of the tracing star at `elapsed` seconds into the
/// Sliding phase (piecewise-linear time→distance across segment breakpoints).
pub fn trace_position(path: &SlidePath, elapsed: f32) -> (Vec2, f32) {
    get_transform_at_distance(&path.waypoints, trace_distance(path, elapsed))
}

/// Map `elapsed` seconds to a distance along the concatenated path.
pub fn trace_distance(path: &SlidePath, elapsed: f32) -> f32 {
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
