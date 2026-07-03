//! Pure geometry toolkit for slide paths: button positions/angles, tangent &
//! blend-arc construction, and polyline sampling. No knowledge of slide shapes
//! or timing — those live in `generators` and `trace`.

use bevy::prelude::Vec2;
use std::f32::consts::TAU;

use super::super::RADIUS;
use super::super::resources::ButtonLayout;

// ── Button geometry ──────────────────────────────────────────────────────────
// All sensor positions and angles are derived from `ButtonLayout` — the single
// source of truth shared with taps/touches — so the slide path can never drift
// out of sync with the rest of the play-field. `ButtonLayout` stores unit-scaled
// ring vectors; multiplying by `RADIUS` gives world positions.

/// Polar angle of a button's spoke, in radians.
pub(super) fn button_angle(button: usize, layout: &ButtonLayout) -> f32 {
    layout.tap[button - 1].to_angle()
}

/// World radius of the A-sensor ring. Slide endpoints sit on the outer rim
/// (the `tap` ring, where taps land), so the slide head and path start coincide.
/// `tap` vectors are unit length, so this is simply `RADIUS`.
pub(super) fn a_ring_radius() -> f32 {
    RADIUS
}

/// A-sensor ring position (slide endpoint) for `button` — on the outer rim.
pub(super) fn a_sensor_pos(button: usize, layout: &ButtonLayout) -> Vec2 {
    layout.tap[button - 1] * RADIUS
}

/// The play-field center.
pub(super) fn center_pos() -> Vec2 {
    Vec2::ZERO
}

/// Button `steps` positions counter-clockwise (increasing index, wrapping 1..=8).
pub(super) fn button_ccw(button: usize, steps: usize) -> usize {
    (button - 1 + steps) % 8 + 1
}

/// Button `steps` positions clockwise (decreasing index, wrapping 1..=8).
pub(super) fn button_cw(button: usize, steps: usize) -> usize {
    ((button - 1) as isize - steps as isize).rem_euclid(8) as usize + 1
}

/// Number of counter-clockwise steps from `start` to `end`.
pub(super) fn ccw_distance(start: usize, end: usize) -> usize {
    (end as isize - start as isize).rem_euclid(8) as usize
}

/// Number of clockwise steps from `start` to `end`.
pub(super) fn cw_distance(start: usize, end: usize) -> usize {
    (start as isize - end as isize).rem_euclid(8) as usize
}

// ── Angle wrapping ───────────────────────────────────────────────────────────

/// Step `e` down by full turns until it is strictly below `anchor` (CW sweep).
pub(super) fn wrap_below(mut e: f32, anchor: f32) -> f32 {
    while e >= anchor {
        e -= TAU;
    }
    e
}

/// Step `e` up by full turns until it is strictly above `anchor` (CCW sweep).
pub(super) fn wrap_above(mut e: f32, anchor: f32) -> f32 {
    while e <= anchor {
        e += TAU;
    }
    e
}

// ── Tangents & blend arcs ────────────────────────────────────────────────────

/// Compute the two tangent points from an external point `p` to a circle
/// **centered at the origin** with radius `r`.
///
/// Returns `(T_ccw, T_cw)` where:
/// - `T_ccw`: tangent point where the arc is on the **CCW (left)** side of the
///   incoming ray from `p`  — `p × T_ccw > 0`
/// - `T_cw`:  tangent point where the arc is on the **CW (right)** side
///   — `p × T_cw < 0`
///
/// Panics (debug) if `p` is inside or on the circle.
pub(super) fn tangent_points_to_circle(p: Vec2, r: f32) -> (Vec2, Vec2) {
    let d_sq = p.length_squared();
    let l = (d_sq - r * r).max(0.0).sqrt(); // tangent length
    let perp = Vec2::new(-p.y, p.x); // p rotated 90° CCW

    // T± = (r²·p ± r·L·p⊥) / |p|²
    let t_ccw = (r * r * p + r * l * perp) / d_sq;
    let t_cw = (r * r * p - r * l * perp) / d_sq;
    (t_ccw, t_cw)
}

/// Compute the two tangent points from an external point `p` to a circle
/// centered at `center` with radius `r`.
///
/// Returns `(T_ccw, T_cw)` in the same convention as [`tangent_points_to_circle`],
/// but relative to the circle's center (not the origin).
pub(super) fn tangent_points_to_offset_circle(p: Vec2, center: Vec2, r: f32) -> (Vec2, Vec2) {
    let p_local = p - center;
    let (t_ccw_local, t_cw_local) = tangent_points_to_circle(p_local, r);
    (center + t_ccw_local, center + t_cw_local)
}

/// Generate a G1-smooth **blend arc** from `p_from` to `tangent_pt`, where
/// `tangent_pt` lies on a circle centred at `circle_center`.
///
/// The blend arc arrives at `tangent_pt` sharing the exact same tangent
/// direction as the loop circle, so the junction has no direction kink.
///
/// Geometry:
///   The blend-arc center must lie on the outward normal of the loop circle at
///   `tangent_pt` (this is the only line whose perpendicular at `tangent_pt`
///   is also tangent to the loop circle).  Combined with the requirement that
///   `p_from` lies on the arc, the center is uniquely determined:
///
/// ```text
///     outward = (tangent_pt - circle_center) / r   (unit outward normal)
///     d       = tangent_pt - p_from                (chord)
///     t       = -|d|^2 / (2 * (d . outward))
///     center  = tangent_pt + t * outward
/// ```
///
/// `ccw` — desired winding of the blend arc (true = CCW, false = CW).
///         Must match the winding of the loop arc that follows/precedes it.
///
/// Falls back to a two-point straight segment on degenerate input.
pub(super) fn blend_arc(
    p_from: Vec2,
    tangent_pt: Vec2,
    circle_center: Vec2,
    ccw: bool,
    spacing: f32,
) -> Vec<Vec2> {
    let outward = (tangent_pt - circle_center).normalize_or_zero();
    let d = tangent_pt - p_from;
    let d_dot_n = d.dot(outward);

    if d_dot_n.abs() < 1e-4 || d.length_squared() < 1e-4 {
        return vec![p_from, tangent_pt];
    }

    let t = -d.length_squared() / (2.0 * d_dot_n);
    let arc_center = tangent_pt + t * outward;
    let arc_radius = arc_center.distance(p_from);

    if arc_radius < 1e-4 {
        return vec![p_from, tangent_pt];
    }

    let start_ang = (p_from - arc_center).to_angle();
    let end_ang_raw = (tangent_pt - arc_center).to_angle();

    // Shift end_ang into the correct half-plane for the requested winding,
    // keeping the arc strictly less than one full revolution.
    let end_ang = if ccw {
        let mut e = end_ang_raw;
        while e <= start_ang {
            e += TAU;
        }
        while e > start_ang + TAU {
            e -= TAU;
        }
        e
    } else {
        let mut e = end_ang_raw;
        while e >= start_ang {
            e -= TAU;
        }
        while e < start_ang - TAU {
            e += TAU;
        }
        e
    };

    generate_offset_arc_points(arc_center, start_ang, end_ang, arc_radius, spacing)
}

// ── Polyline sampling ────────────────────────────────────────────────────────

/// Calculate the total physical distance along a polyline path.
pub fn calculate_total_length(points: &[Vec2]) -> f32 {
    let mut length = 0.0;
    for window in points.windows(2) {
        length += window[0].distance(window[1]);
    }
    length
}

/// Find the exact position and facing angle at a specific distance along a polyline path.
pub fn get_transform_at_distance(points: &[Vec2], target_distance: f32) -> (Vec2, f32) {
    let mut current_dist = 0.0;

    for window in points.windows(2) {
        let p1 = window[0];
        let p2 = window[1];
        let segment_length = p1.distance(p2);

        if segment_length < f32::EPSILON {
            continue;
        }

        if current_dist + segment_length >= target_distance {
            let t = (target_distance - current_dist) / segment_length;
            let exact_pos = p1.lerp(p2, t);
            let angle = (p2.y - p1.y).atan2(p2.x - p1.x);
            return (exact_pos, angle);
        }
        current_dist += segment_length;
    }

    // Fallback: return the last point
    if points.len() >= 2 {
        let last = points[points.len() - 1];
        let prev = points[points.len() - 2];
        let angle = (last.y - prev.y).atan2(last.x - prev.x);
        (last, angle)
    } else if let Some(p) = points.last() {
        (*p, 0.0)
    } else {
        (Vec2::ZERO, 0.0)
    }
}

/// Append `next_segment` to `base`, skipping the first point if it duplicates
/// the last point of `base` (within 1.0 units).
pub(super) fn append_path_dedup(base: &mut Vec<Vec2>, mut next_segment: Vec<Vec2>) {
    if let (Some(last), Some(first)) = (base.last(), next_segment.first()) {
        if last.distance(*first) < 1.0 {
            next_segment.remove(0);
        }
    }
    base.extend(next_segment);
}

// ── Arc & polyline point generation ──────────────────────────────────────────

/// Generate evenly-spaced points along a circular arc centred at the origin.
pub fn generate_arc_points(
    start_angle: f32,
    end_angle: f32,
    radius: f32,
    spacing: f32,
) -> Vec<Vec2> {
    generate_offset_arc_points(Vec2::ZERO, start_angle, end_angle, radius, spacing)
}

/// Generate evenly-spaced points along an arc centered at an arbitrary point.
pub fn generate_offset_arc_points(
    center: Vec2,
    start_angle: f32,
    end_angle: f32,
    radius: f32,
    spacing: f32,
) -> Vec<Vec2> {
    let angle_diff = (end_angle - start_angle).abs();
    let arc_length = radius * angle_diff;
    let steps = (arc_length / spacing).ceil().max(1.0) as usize;
    let mut points = Vec::with_capacity(steps + 1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let current_angle = start_angle + (end_angle - start_angle) * t;
        points.push(center + Vec2::new(radius * current_angle.cos(), radius * current_angle.sin()));
    }
    points
}

/// Generate evenly-spaced points along a multi-segment polyline.
pub fn generate_multi_segment_points(waypoints: &[Vec2], spacing: f32) -> Vec<Vec2> {
    let mut points = Vec::new();

    for w in 0..(waypoints.len() - 1) {
        let p1 = waypoints[w];
        let p2 = waypoints[w + 1];
        let distance = p1.distance(p2);
        let steps = (distance / spacing).ceil().max(1.0) as usize;

        let start_i = if w == 0 { 0 } else { 1 };

        for i in start_i..=steps {
            let t = i as f32 / steps as f32;
            points.push(p1.lerp(p2, t));
        }
    }
    points
}
