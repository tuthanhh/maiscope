//! Per-shape waypoint generators. Each `generate_shape_*` turns a slide shape
//! plus its start/end buttons into a polyline; `generate_points` dispatches on
//! the [`SlideShape`] variant. All geometry primitives come from [`super::geometry`].

use bevy::math::ops::tan;
use bevy::prelude::Vec2;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_8, PI, TAU};

use super::super::CHEVRON_SPACING;
use super::super::resources::ButtonLayout;
use super::geometry::*;
use crate::systems::component::SlideShape;

// cos(3*pi/8) = sin(pi/8) ≈ 0.3827 — natural tangent-circle radius for P/Q inner loop
const INNER_RING_FRAC: f32 = 0.382_683_43;

/// Generate the waypoint path for a single slide segment.
///
/// The path follows the actual maimai touch sensor positions as documented
/// in the official slide touch sensor usage guide. The segment's start button
/// is supplied separately (it comes from the slide head, or the previous
/// segment's end); `SlideShape` only carries the segment's end(s).
///
/// # Arguments
/// * `shape` - The slide shape to generate
/// * `start_button` - 1-based button the segment starts from
/// * `layout` - The button layout providing sensor ring positions/angles
pub fn generate_points(
    shape: &SlideShape,
    start_button: usize,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let s = start_button;
    match shape {
        SlideShape::Straight { end } => generate_shape_straight(s, *end, layout),
        SlideShape::ShortArc { end } => generate_shape_short_arc(s, *end, layout),
        SlideShape::ClockwiseArc { end } => generate_directional_arc(s, *end, true, layout),
        SlideShape::CounterClockwiseArc { end } => generate_directional_arc(s, *end, false, layout),
        SlideShape::VShape { end } => generate_shape_v(s, *end, layout),
        SlideShape::GrandVShape { mid, end } => generate_shape_grand_v(s, *end, *mid, layout),
        SlideShape::PShape { end } => generate_shape_inner_loop(s, *end, false, layout),
        SlideShape::QShape { end } => generate_shape_inner_loop(s, *end, true, layout),
        SlideShape::GrandPShape { end } => generate_shape_grand_loop(s, *end, false, layout),
        SlideShape::GrandQShape { end } => generate_shape_grand_loop(s, *end, true, layout),
        SlideShape::Thunderbolt { end, is_z } => generate_shape_thunderbolt(s, *end, *is_z, layout),
        SlideShape::FanShape {
            ends: (e1, _e2, _e3),
        } => generate_shape_fan(s, *e1, layout),
    }
}

// STRAIGHT (-): Direct line from start A-sensor to end A-sensor.
fn generate_shape_straight(start: usize, end: usize, layout: &ButtonLayout) -> Vec<Vec2> {
    let p1 = a_sensor_pos(start, layout);
    let p2 = a_sensor_pos(end, layout);
    generate_multi_segment_points(&[p1, p2], 35.0)
}

// SHORT ARC (^): The shortest arc along the boundary ring (never > 180°).
fn generate_shape_short_arc(start: usize, end: usize, layout: &ButtonLayout) -> Vec<Vec2> {
    let r = a_ring_radius();
    let start_ang = button_angle(start, layout);
    let end_ang = button_angle(end, layout);

    let mut diff = (end_ang - start_ang) % TAU;
    if diff > PI {
        diff -= TAU;
    }
    if diff < -PI {
        diff += TAU;
    }
    let target = start_ang + diff;

    generate_arc_points(start_ang, target, r, 35.0)
}

// CLOCKWISE / COUNTER-CLOCKWISE ARC (> / <): arc along the boundary ring.
// Direction is chosen by the start button's half: top-half buttons {1,2,7,8}
// sweep one way, bottom-half {3,4,5,6} the other, so the on-screen meaning of
// the symbol stays consistent. Same-button start==end yields a full circle.
fn generate_directional_arc(
    start: usize,
    end: usize,
    is_clockwise: bool,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let spacing = CHEVRON_SPACING;
    let r = a_ring_radius();
    let start_ang = button_angle(start, layout);
    let end_ang = button_angle(end, layout);

    let top_half = matches!(start, 1 | 2 | 7 | 8);
    let end_ang = if top_half == is_clockwise {
        wrap_below(end_ang, start_ang)
    } else {
        wrap_above(end_ang, start_ang)
    };
    generate_arc_points(start_ang, end_ang, r, spacing)
}

// V-SHAPE (v): start A-sensor → center → end A-sensor.
fn generate_shape_v(start: usize, end: usize, layout: &ButtonLayout) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, layout);
    let p_center = center_pos();
    let p_end = a_sensor_pos(end, layout);
    generate_multi_segment_points(&[p_start, p_center, p_end], 35.0)
}

// GRAND V-SHAPE (V): start A-sensor → mid A-sensor → end A-sensor.
fn generate_shape_grand_v(
    start: usize,
    end: usize,
    mid: usize,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, layout);
    let p_mid = a_sensor_pos(mid, layout);
    let p_end = a_sensor_pos(end, layout);
    generate_multi_segment_points(&[p_start, p_mid, p_end], 35.0)
}

// INNER LOOP — P (p, CCW) and Q (q, CW): tangent to the R/3 inner circle.
//
// Entry and exit use opposite tangent sides:
//   P (CCW arc): entry = T_ccw(start), exit = T_cw(end)
//   Q (CW  arc): entry = T_cw(start),  exit = T_ccw(end)
fn generate_shape_inner_loop(
    start: usize,
    end: usize,
    is_q: bool,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let spacing = CHEVRON_SPACING;
    let r_inner = a_ring_radius() * INNER_RING_FRAC;

    let p_start = a_sensor_pos(start, layout);
    let p_end = a_sensor_pos(end, layout);

    let (t_ccw_entry, t_cw_entry) = tangent_points_to_circle(p_start, r_inner);
    let (t_ccw_exit, t_cw_exit) = tangent_points_to_circle(p_end, r_inner);

    let tangent_entry = if is_q { t_cw_entry } else { t_ccw_entry };
    let tangent_exit = if is_q { t_ccw_exit } else { t_cw_exit };

    let arc_start_ang = tangent_entry.to_angle();
    let arc_end_ang_raw = tangent_exit.to_angle();

    // The minimal positive sweep (CCW for P, CW for Q) is the natural loop; an
    // extra full turn would over-rotate into a second circle.
    let arc_end_ang = if is_q {
        wrap_below(arc_end_ang_raw, arc_start_ang)
    } else {
        wrap_above(arc_end_ang_raw, arc_start_ang)
    };

    let mut path = blend_arc(p_start, tangent_entry, Vec2::ZERO, !is_q, spacing);
    append_path_dedup(
        &mut path,
        generate_arc_points(arc_start_ang, arc_end_ang, r_inner, spacing),
    );

    let mut exit_seg = blend_arc(p_end, tangent_exit, Vec2::ZERO, is_q, spacing);
    exit_seg.reverse();
    append_path_dedup(&mut path, exit_seg);

    path
}

// OUTER LOOP — PP (pp, CCW) and QQ (qq, CW): loop through a circle of radius R/2
// positioned 90° perpendicular to the start spoke. Exit waypoint varies by
// distance from start to end.
fn generate_shape_grand_loop(
    start: usize,
    end: usize,
    is_q: bool,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let spacing = CHEVRON_SPACING;
    let r_loop = a_ring_radius() * 0.5;
    let start_ang = button_angle(start, layout);

    // Loop circle sits 90° to the side of the spoke (internally tangent to boundary).
    let center_ang = if is_q {
        start_ang + FRAC_PI_2
    } else {
        start_ang - FRAC_PI_2
    };
    let loop_center = Vec2::new(r_loop * center_ang.cos(), r_loop * center_ang.sin());

    let p_start = a_sensor_pos(start, layout);
    let a_end = a_sensor_pos(end, layout);

    let (t_ccw_entry, t_cw_entry) = tangent_points_to_offset_circle(p_start, loop_center, r_loop);
    let tangent_entry = if is_q { t_cw_entry } else { t_ccw_entry };
    let arc_entry_ang = (tangent_entry - loop_center).to_angle();

    let exit_waypoint = a_sensor_pos(end, layout);
    let (t_ccw_exit, t_cw_exit) =
        tangent_points_to_offset_circle(exit_waypoint, loop_center, r_loop);
    let tangent_exit = if is_q { t_ccw_exit } else { t_cw_exit };
    let arc_exit_ang = (tangent_exit - loop_center).to_angle();

    let target_arc_ang = if is_q {
        wrap_below(arc_exit_ang, arc_entry_ang - FRAC_PI_4)
    } else {
        wrap_above(arc_exit_ang, arc_entry_ang + FRAC_PI_4)
    };

    let mut path = blend_arc(p_start, tangent_entry, loop_center, !is_q, spacing);
    append_path_dedup(
        &mut path,
        generate_offset_arc_points(loop_center, arc_entry_ang, target_arc_ang, r_loop, spacing),
    );

    let exit_target = if exit_waypoint.distance(a_end) > 1.0 {
        exit_waypoint
    } else {
        a_end
    };
    let mut exit_seg = blend_arc(exit_target, tangent_exit, loop_center, is_q, spacing);
    exit_seg.reverse();
    append_path_dedup(&mut path, exit_seg);

    if exit_waypoint.distance(a_end) > 1.0 {
        append_path_dedup(
            &mut path,
            generate_multi_segment_points(&[exit_waypoint, a_end], spacing),
        );
    }

    path
}

// THUNDERBOLT / ZIGZAG (s / z): a 5-point lightning bolt
// `start → tip1 → center → tip2 → end`.
//
// The two tips are anchored one button step off the *opposite* endpoint and
// pulled `tan(π/8)` of the way back toward the *near* endpoint, which seats them
// just inside the ring and gives the bolt its slanted kinks:
//   tip1: anchor = neighbour of `end`,   pulled toward `start`
//   tip2: anchor = neighbour of `start`, pulled toward `end`
// `s` steps counter-clockwise to the neighbour, `z` clockwise.
fn generate_shape_thunderbolt(
    start: usize,
    end: usize,
    is_z: bool,
    layout: &ButtonLayout,
) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, layout);
    let p_end = a_sensor_pos(end, layout);
    let p_center = center_pos();

    let step = |b| {
        if is_z {
            button_cw(b, 1)
        } else {
            button_ccw(b, 1)
        }
    };
    let tip1_anchor = step(end);
    let tip2_anchor = step(start);

    let tip1 = a_sensor_pos(tip1_anchor, layout).lerp(p_start, tan(FRAC_PI_8));
    let tip2 = a_sensor_pos(tip2_anchor, layout).lerp(p_end, tan(FRAC_PI_8));

    generate_multi_segment_points(&[p_start, tip1, p_center, tip2, p_end], 35.0)
}

// FAN SHAPE (w): Path to first end only. The spawner creates 3 separate
// simultaneous slides for end1, end2, end3.
fn generate_shape_fan(start: usize, end1: usize, layout: &ButtonLayout) -> Vec<Vec2> {
    let p_start = a_sensor_pos(start, layout);
    let p_end = a_sensor_pos(end1, layout);
    generate_multi_segment_points(&[p_start, p_end], 35.0)
}
