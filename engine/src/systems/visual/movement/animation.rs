//! Stateless per-element animators driven by `update_movement`. Each function
//! advances one note's visual elements for a given phase fraction `t`; the
//! phase state machine that calls them lives in [`super`].

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::systems::{
    MOVING,
    component::{Duration, NoteKind},
    visual::{
        NOTE_RADIUS, RADIUS, SPARK_STAR_RADIUS,
        component::{FanLanes, HoldNoteElement, SlideElement, SlidePath, TouchElement, TouchSpark},
        resources::ButtonLayout,
        shapes, slide_path,
    },
};

use super::{
    CountdownQuery, HaloHoldQuery, HoldElementQuery, NoteHaloQuery, SlideArrowQuery,
    SlideElementQuery, TouchSparkQuery, TriangleQuery,
};

// ── Small shared helpers ─────────────────────────────────────────────────────

pub(super) fn duration_to_secs(duration: Duration, bpm: f32) -> f32 {
    match duration {
        Duration::Simple { divider, count } => count as f32 / divider as f32 * (240.0 / bpm),
        Duration::BpmOverride {
            bpm,
            divider,
            count,
        } => count as f32 / divider as f32 * (240.0 / bpm),
        Duration::BpmOverrideSeconds { seconds, .. } => seconds,
        _ => 0.0,
    }
}

/// True hold-bar length: how far the note travels during the hold,
/// `velocity_wall · hold_wall`. With `move_duration = MOVING/(chart·note)` the
/// wall velocity is `travel_dist·chart·note/MOVING` and the wall hold time is
/// `dur_secs/chart` — the `chart_speed` cancels, leaving a function of
/// `note_speed` only.
fn hold_max_tail(travel_dist: f32, note_speed: f32, duration: Duration, bpm: f32) -> f32 {
    travel_dist * note_speed / MOVING as f32 * duration_to_secs(duration, bpm)
}

pub(super) fn set_alpha(sprite: &mut Sprite, a: f32) {
    sprite.color.set_alpha(a);
}

/// Fade a slide-arrow chevron, which is a sprite (ordinary slide) or a lyon
/// `Shape` (fan/wifi cone) — whichever component the arrow carries.
fn set_arrow_alpha(sprite: Option<Mut<Sprite>>, shape: Option<Mut<Shape>>, a: f32) {
    if let Some(mut sprite) = sprite {
        sprite.color.set_alpha(a);
    }
    if let Some(mut shape) = shape {
        if let Some(fill) = shape.fill.as_mut() {
            fill.color.set_alpha(a);
        }
        if let Some(stroke) = shape.stroke.as_mut() {
            stroke.color.set_alpha(a);
        }
    }
}

/// True for slide notes (with or without a head star).
pub(super) fn is_slide(kind: &NoteKind) -> bool {
    matches!(
        kind,
        NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. }
    )
}

/// `(spawn, hit)` world positions for button `id`: where the note appears and
/// where it lands on the outer ring.
fn travel_endpoints(layout: &ButtonLayout, id: usize) -> (Vec2, Vec2) {
    let spawn = layout.tap_spawn[id - 1] * RADIUS;
    let hit = layout.tap[id - 1] * RADIUS;
    (spawn, hit)
}

/// Halo pulse: a 0.25s sawtooth that repeats for the whole hold — scale ramps
/// 0.75 -> 1.75 and alpha 0.3 -> 0.5, then snaps back.
fn pulse_halo(sprite: &mut Sprite, transform: &mut Transform, timer: &Timer) {
    let cycle = (timer.elapsed_secs() % 0.25) / 0.25;
    transform.scale = Vec3::splat(0.75 + cycle); // 0.75 -> 1.75
    set_alpha(sprite, 0.3 + cycle * 0.2); // 0.3 -> 0.5
}

// ── Growing phase ────────────────────────────────────────────────────────────

pub(super) fn grow_slide(
    t: f32,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    slide_arrows: &mut SlideArrowQuery,
    note_halos: &mut NoteHaloQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, _, _, head_children)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                transform.scale = Vec3::splat(t);
                scale_note_halo(&transform, head_children, note_halos);
            }
        }
        if let Ok((sprite, shape, _)) = slide_arrows.get_mut(child) {
            set_arrow_alpha(sprite, shape, t);
        }
    }
}

/// Keep a note's faint halo (`Normal.png`) concentric with the judgement circle.
///
/// The halo's glow dot is its anchor and rides the note, so the ring is centred
/// on the world origin only when its radius equals the note's current distance.
/// Setting the child scale to `distance / RADIUS` makes the ring grow linearly
/// from the spawn circle (≈0.3·R) to the judgement ring (R) as the note travels.
/// Dividing by the parent's grow-scale cancels the note's 0→1 pop-in, so the
/// ring tracks true world distance rather than the shrinking note body.
pub(super) fn scale_note_halo(
    note: &Transform,
    children: Option<&Children>,
    halos: &mut NoteHaloQuery,
) {
    let Some(children) = children else { return };
    let parent_scale = note.scale.x;
    if parent_scale <= f32::EPSILON {
        return;
    }
    let ring_scale = note.translation.truncate().length() / RADIUS / parent_scale;
    for child in children.iter() {
        if let Ok((mut tf, _)) = halos.get_mut(child) {
            tf.scale = Vec3::splat(ring_scale);
        }
    }
}

// ── Moving phase ─────────────────────────────────────────────────────────────

pub(super) fn move_tap(transform: &mut Transform, kind: &NoteKind, t: f32, layout: &ButtonLayout) {
    if let NoteKind::Tap(id) | NoteKind::SlideStar(id) = kind {
        let (spawn, hit) = travel_endpoints(layout, *id);
        transform.translation = spawn.lerp(hit, t).extend(2.0);
    }
}

pub(super) fn move_taphold(
    kind: &NoteKind,
    t: f32,
    bpm: f32,
    note_speed: f32,
    children: Option<&Children>,
    transform: &mut Transform,
    hold_elements: &mut HoldElementQuery,
    layout: &ButtonLayout,
) {
    let NoteKind::TapHold { button, duration } = kind else {
        return;
    };

    let Some(children) = children else { return };

    let (spawn, hit) = travel_endpoints(layout, *button);
    transform.translation = spawn.lerp(hit, t).extend(2.0);

    let travel_dist = spawn.distance(hit);
    let max_tail = hold_max_tail(travel_dist, note_speed, *duration, bpm);
    let current_length = (travel_dist * t).min(max_tail);

    for child in children.iter() {
        if let Ok((mut tf, el)) = hold_elements.get_mut(child) {
            match el {
                HoldNoteElement::Body => {
                    tf.scale.y = current_length;
                    tf.translation.y = -current_length / 2.0;
                }
                HoldNoteElement::Tail | HoldNoteElement::EndDot => {
                    tf.translation.y = -current_length;
                }
                HoldNoteElement::Head => {}
            }
        }
    }
}

pub(super) fn move_slide(
    t: f32,
    kind: &NoteKind,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    layout: &ButtonLayout,
    note_halos: &mut NoteHaloQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, _, _, head_children)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                if let NoteKind::Slide {
                    head_button: id, ..
                } = kind
                {
                    // Land on the outer rim, coinciding with the path start.
                    let (spawn, hit) = travel_endpoints(layout, *id);
                    transform.translation = spawn.lerp(hit, t).extend(2.0);
                    scale_note_halo(&transform, head_children, note_halos);
                }
            }
        }
    }
}

pub(super) fn hide_slide_head(children: Option<&Children>, slide_elements: &mut SlideElementQuery) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((_t, el, mut vis, _s, _)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::Head) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

pub(super) fn move_triangles(
    kind: &NoteKind,
    t: f32,
    children: Option<&Children>,
    triangles: &mut TriangleQuery,
) {
    if !matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
        return;
    }
    let Some(children) = children else { return };
    let current_dist = shapes::touch_triangle_start_distance(NOTE_RADIUS) * (1.0 - t);
    for child in children.iter() {
        if let Ok((mut tf, element)) = triangles.get_mut(child) {
            if matches!(element, TouchElement::Triangle) {
                let dir = tf.translation.truncate().normalize_or_zero();
                tf.translation = (dir * current_dist).extend(-0.1);
            }
        }
    }
}

// ── Touch death burst ────────────────────────────────────────────────────────

/// Touch death burst, driven by the parent `Dying` fraction `t`:
/// the halo expands and fades across the whole effect; 8 stars converge to the
/// center in the first sub-phase; 4 stars burst outward in the second.
pub(super) fn animate_touch_spark(
    t: f32,
    children: Option<&Children>,
    sparks: &mut TouchSparkQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        let Ok((mut transform, mut shape, spark)) = sparks.get_mut(child) else {
            continue;
        };
        match *spark {
            TouchSpark::Halo => {
                transform.scale = Vec3::splat(0.4 + t * 1.0); // 0.4 -> 2.2
                set_alpha(&mut shape, 1.0 - t);
            }
            TouchSpark::StarIn(angle) => {
                let f = (t / 0.55).min(1.0);
                let r = SPARK_STAR_RADIUS * (1.0 - f);
                let pos = Vec2::new(angle.cos(), angle.sin()) * r;
                transform.translation = pos.extend(2.0);
                set_alpha(&mut shape, 1.0 - f);
            }
            TouchSpark::StarOut(angle) => {
                let f = ((t - 0.45) / 0.55).clamp(0.0, 1.0);
                let r = SPARK_STAR_RADIUS * f;
                let pos = Vec2::new(angle.cos(), angle.sin()) * r;
                transform.translation = pos.extend(2.0);
                set_alpha(&mut shape, (f * std::f32::consts::PI).sin()); // fade in then out
            }
        }
    }
}

// ── Holding phase ────────────────────────────────────────────────────────────

pub(super) fn hold_tap(
    kind: &NoteKind,
    t: f32,
    timer: &Timer,
    bpm: f32,
    note_speed: f32,
    children: Option<&Children>,
    hold_elements: &mut HoldElementQuery,
    halo_holds: &mut HaloHoldQuery,
    layout: &ButtonLayout,
) {
    let NoteKind::TapHold { button, duration } = kind else {
        return;
    };
    let Some(children) = children else { return };

    let (spawn, hit) = travel_endpoints(layout, *button);

    let travel_dist = spawn.distance(hit);
    let max_tail = hold_max_tail(travel_dist, note_speed, *duration, bpm);
    let current_length = (max_tail * (1.0 - t)).min(travel_dist);

    for child in children.iter() {
        if let Ok((mut tf, el)) = hold_elements.get_mut(child) {
            match el {
                HoldNoteElement::Body => {
                    tf.scale.y = current_length;
                    tf.translation.y = -current_length / 2.0;
                }
                HoldNoteElement::Tail | HoldNoteElement::EndDot => {
                    tf.translation.y = -current_length;
                }
                HoldNoteElement::Head => {}
            }
        }
        if let Ok((mut shape, mut transform, _)) = halo_holds.get_mut(child) {
            pulse_halo(&mut shape, &mut transform, timer);
        }
    }
}

pub(super) fn hold_touch(
    t: f32,
    timer: &Timer,
    children: Option<&Children>,
    countdown_query: &mut CountdownQuery,
    halo_holds: &mut HaloHoldQuery,
) {
    let Some(children) = children else { return };
    let edges = shapes::COUNTDOWN_EDGES as f32;
    for child in children.iter() {
        // Draw each coloured edge of the diamond in turn, clockwise: edge `e`
        // fills over the `t` window `[e/4, (e+1)/4]`, so the border is revealed
        // only as far as the countdown has progressed.
        if let Ok((mut shape, mut vis, countdown)) = countdown_query.get_mut(child) {
            let edge = countdown.edge;
            let frac = (t * edges - edge as f32).clamp(0.0, 1.0);
            *vis = Visibility::Visible;
            shape.path = ShapeBuilder::with(&shapes::countdown_edge(
                edge,
                shapes::COUNTDOWN_SWEEP_S,
                frac,
            ))
            .stroke((shapes::COUNTDOWN_EDGE_COLORS[edge], NOTE_RADIUS * 0.18))
            .build()
            .path;
        }
        if let Ok((mut shape, mut transform, _)) = halo_holds.get_mut(child) {
            pulse_halo(&mut shape, &mut transform, timer);
        }
    }
}

// ── Waiting / Sliding phases ─────────────────────────────────────────────────

/// Waiting phase: the trace star fades in, stationary at the path start.
pub(super) fn wait_slide(
    t: f32,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, mut vis, mut shape, _)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::TraceStar(_)) {
                *vis = Visibility::Visible;
                transform.scale = Vec3::splat(t);
                set_alpha(&mut shape, t);
            }
        }
    }
}

/// Sliding phase: the trace star walks the path; chevrons it passes are removed.
pub(super) fn slide_trace(
    frac: f32,
    path: &SlidePath,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    slide_arrows: &mut SlideArrowQuery,
    commands: &mut Commands,
) {
    let Some(children) = children else { return };
    let dist = frac * path.total_length;
    let (pos, angle) = slide_path::get_transform_at_distance(&path.waypoints, dist);
    for child in children.iter() {
        if let Ok((mut transform, el, _vis, mut shape, _)) = slide_elements.get_mut(child) {
            if matches!(*el, SlideElement::TraceStar(_)) {
                set_alpha(&mut shape, 1.0);
                transform.translation = pos.extend(3.0);
                transform.rotation = Quat::from_rotation_z(angle);
            }
        }
        if let Ok((_, _, arrow)) = slide_arrows.get_mut(child) {
            if arrow.distance_along_path <= dist {
                commands.entity(child).despawn();
            }
        }
    }
}

/// Fan Sliding phase: each lane's trace star walks its own lane at the shared
/// progress fraction; chevrons are removed as their lane's star passes them.
pub(super) fn fan_trace(
    frac: f32,
    fan: &FanLanes,
    children: Option<&Children>,
    slide_elements: &mut SlideElementQuery,
    slide_arrows: &mut SlideArrowQuery,
    commands: &mut Commands,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if let Ok((mut transform, el, _vis, mut shape, _)) = slide_elements.get_mut(child) {
            if let SlideElement::TraceStar(lane) = *el {
                let len = fan.lengths.get(lane).copied().unwrap_or(0.0);
                let (pos, angle) =
                    slide_path::get_transform_at_distance(&fan.lanes[lane], frac * len);
                set_alpha(&mut shape, 1.0);
                transform.translation = pos.extend(3.0);
                transform.rotation = Quat::from_rotation_z(angle);
            }
        }
        if let Ok((_, _, arrow)) = slide_arrows.get_mut(child) {
            let len = fan.lengths.get(arrow.lane).copied().unwrap_or(0.0);
            if arrow.distance_along_path <= frac * len {
                commands.entity(child).despawn();
            }
        }
    }
}
