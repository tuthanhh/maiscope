//! Note lifecycle driver. `update_movement` runs every frame and advances each
//! note through its `NoteTiming` phases (Growing → Moving → Holding/Waiting →
//! Sliding → Dying), delegating per-element animation to [`animation`].

use super::spawning::spawn_touch_spark;
use super::{RADIUS, component::NoteTiming, resources::ButtonLayout, resources::NoteAssets};
use crate::systems::{
    MOVING,
    chart_playback::ChartPlayback,
    component::NoteKind,
    visual::{
        component::{
            FanLanes, HoldHalo, HoldNoteElement, NoteBpm, NoteHalo, SlideArrow, SlideElement,
            SlidePath, TouchElement, TouchHoldCountdown, TouchSpark,
        },
        shapes::{hexagon_sprite, hold_halo_sprite, touch_just_sprite},
        slide_path,
    },
};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

mod animation;
use crate::systems::audio::PlayGuideSoundMessage;
use animation::*;

type TriangleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static TouchElement),
    (
        Without<NoteTiming>,
        Without<HoldNoteElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
    ),
>;

type HoldElementQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static HoldNoteElement),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<SlideElement>,
    ),
>;
type HaloHoldQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static HoldHalo,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<SlideElement>,
        Without<HoldNoteElement>,
    ),
>;
// Faint note halo rings (tap/star). Holds a unique `NoteHalo` marker and
// excludes every other animated marker, so its `&mut Transform` access is
// provably disjoint from all the queries above.
type NoteHaloQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static NoteHalo),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<HoldNoteElement>,
        Without<HoldHalo>,
        Without<TouchHoldCountdown>,
        Without<SlideElement>,
        Without<SlideArrow>,
        Without<TouchSpark>,
    ),
>;
// Coloured countdown-border edges. Each carries `&mut Shape` (redrawn for the
// clockwise reveal) + `&mut Visibility`; disjoint from every other query via its
// unique `TouchHoldCountdown` marker.
type CountdownQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Shape,
        &'static mut Visibility,
        &'static TouchHoldCountdown,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<SlideElement>,
        Without<SlideArrow>,
        Without<HoldNoteElement>,
    ),
>;

type SlideElementQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static SlideElement,
        &'static mut Visibility,
        &'static mut Sprite,
        // The head star's concentric halo child (absent on trace stars/chevrons).
        Option<&'static Children>,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideArrow>,
        Without<HoldNoteElement>,
        Without<NoteHalo>,
    ),
>;
type SlideArrowQuery<'w, 's> = Query<
    'w,
    's,
    // Ordinary slides carry a sprite chevron; fan/wifi cones carry a lyon
    // `Shape`. Exactly one is present per arrow.
    (
        Option<&'static mut Sprite>,
        Option<&'static mut Shape>,
        &'static mut SlideArrow,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<TouchHoldCountdown>,
        Without<SlideElement>,
        Without<HoldNoteElement>,
    ),
>;
// Disjoint from every other query above: each requires a unique marker that the
// spark children lack, so excluding all of them proves no aliasing.
type TouchSparkQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Sprite,
        &'static TouchSpark,
    ),
    (
        Without<NoteTiming>,
        Without<TouchElement>,
        Without<HoldNoteElement>,
        Without<HoldHalo>,
        Without<TouchHoldCountdown>,
        Without<SlideElement>,
        Without<SlideArrow>,
    ),
>;

pub fn update_movement(
    mut commands: Commands,
    mut entity_query: Query<(
        Entity,
        Option<&mut Sprite>,
        &mut Transform,
        &mut NoteTiming,
        &NoteKind,
        &mut Visibility,
        Option<&Children>,
        Option<&NoteBpm>,
        Option<&SlidePath>,
        Option<&FanLanes>,
    )>,
    mut triangles: TriangleQuery,
    mut hold_elements: HoldElementQuery,
    mut countdown_query: CountdownQuery,
    mut slide_elements: SlideElementQuery,
    mut slide_arrows: SlideArrowQuery,
    mut halo_holds: HaloHoldQuery,
    mut note_halos: NoteHaloQuery,
    mut touch_sparks: TouchSparkQuery,
    chart: Res<ChartPlayback>,
    layout: Res<ButtonLayout>,
    time: Res<Time>,
    assets: Res<NoteAssets>,

    mut guide_sound_messages: MessageWriter<PlayGuideSoundMessage>,
) {
    // Travel is faster with both play speed and note speed; hold/slide musical
    // durations follow the play tempo (/ chart_speed), so effective bpm is
    // bpm × chart_speed.
    let note_speed = chart.note_speed();
    let chart_speed = chart.chart_speed();
    let move_duration = MOVING as f32 / (chart_speed * note_speed);

    for (
        entity,
        mut sprite,
        mut transform,
        mut timing,
        kind,
        mut visibility,
        children,
        note_bpm,
        slide_path_data,
        fan_lanes,
    ) in entity_query.iter_mut()
    {
        match &mut *timing {
            NoteTiming::Growing(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();

                if !is_slide(kind) {
                    transform.scale = Vec3::splat(t);
                    scale_note_halo(&transform, children, &mut note_halos);
                } else {
                    grow_slide(
                        t,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut note_halos,
                    );
                }

                if timer.just_finished() {
                    *timing =
                        NoteTiming::Moving(Timer::from_seconds(move_duration, TimerMode::Once));
                    if matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
                        *visibility = Visibility::Visible;
                    }
                }
            }
            NoteTiming::Moving(timer) => {
                timer.tick(time.delta());
                let t = timer.fraction();

                move_tap(&mut transform, kind, t, &layout);
                scale_note_halo(&transform, children, &mut note_halos);
                move_triangles(kind, t, children, &mut triangles);
                move_slide(
                    t,
                    kind,
                    children,
                    &mut slide_elements,
                    &layout,
                    &mut note_halos,
                );
                move_taphold(
                    kind,
                    t,
                    note_bpm.map(|b| b.0).unwrap_or(0.0),
                    note_speed,
                    children,
                    &mut transform,
                    &mut hold_elements,
                    &layout,
                );

                if timer.just_finished() {
                    match kind {
                        // Touch flashes `TouchJust` for a brief Holding phase, then
                        // bursts in Dying.
                        NoteKind::Touch { .. } => {
                            *timing =
                                NoteTiming::Holding(Timer::from_seconds(0.05, TimerMode::Once));
                        }
                        NoteKind::Tap(_) | NoteKind::SlideStar { .. } => {
                            *timing = NoteTiming::Dying(Timer::from_seconds(0.25, TimerMode::Once));
                        }
                        NoteKind::TapHold { duration, .. }
                        | NoteKind::TouchHold { duration, .. } => {
                            if let Some(NoteBpm(bpm)) = note_bpm {
                                *timing = NoteTiming::Holding(Timer::from_seconds(
                                    duration_to_secs(*duration, *bpm) / chart_speed,
                                    TimerMode::Once,
                                ));

                                guide_sound_messages.write(PlayGuideSoundMessage);
                            }
                        }
                        NoteKind::Slide { .. } | NoteKind::HeadlessSlide { .. } => {
                            hide_slide_head(children, &mut slide_elements);
                            // Pop effect where the head star lands on the ring.
                            if let NoteKind::Slide { head_button, .. } = kind {
                                let pos = layout.tap[head_button - 1] * RADIUS;
                                commands.spawn((
                                    hexagon_sprite(&assets),
                                    Transform::from_translation(pos.extend(2.5)),
                                    Visibility::Visible,
                                    NoteKind::Tap(*head_button),
                                    NoteTiming::Dying(Timer::from_seconds(0.25, TimerMode::Once)),
                                ));
                            }
                            // Musical timing → follows the play tempo (/ chart_speed).
                            let wait =
                                slide_path_data.map(|p| p.wait_secs).unwrap_or(0.0) / chart_speed;
                            *timing =
                                NoteTiming::Waiting(Timer::from_seconds(wait, TimerMode::Once));
                        }
                    }
                }
            }
            NoteTiming::Holding(timer) => {
                if timer.elapsed().is_zero() {
                    match kind {
                        // Hold notes: glowing halo around the head, to make it
                        // visually distinct from a regular tap.
                        NoteKind::TapHold { .. } | NoteKind::TouchHold { .. } => {
                            commands.entity(entity).with_children(|parent| {
                                parent.spawn((hold_halo_sprite(&assets), HoldHalo));
                            });
                        }
                        // Touch: flash `TouchJust` as a new child for the brief
                        // Holding phase; the converged approach triangles stay put
                        // (all are cleared together when Dying begins).
                        NoteKind::Touch { .. } => {
                            guide_sound_messages.write(PlayGuideSoundMessage);
                            commands.entity(entity).with_children(|parent| {
                                parent.spawn((
                                    touch_just_sprite(&assets),
                                    Transform::from_xyz(0.0, 0.0, 0.5),
                                ));
                            });
                        }
                        _ => {}
                    }
                }
                timer.tick(time.delta());

                let t = timer.fraction();

                match kind {
                    NoteKind::TapHold { .. } => {
                        hold_tap(
                            kind,
                            t,
                            timer,
                            note_bpm.map(|b| b.0).unwrap_or(0.0),
                            note_speed,
                            children,
                            &mut hold_elements,
                            &mut halo_holds,
                            &layout,
                        );
                    }
                    NoteKind::TouchHold { .. } => {
                        hold_touch(t, timer, children, &mut countdown_query, &mut halo_holds);
                    }
                    _ => {}
                }

                if timer.just_finished() {
                    // Touch plays the two-sub-phase burst, so it needs the longer
                    // Dying timer; hold notes just pop briefly.
                    let dying_secs = match kind {
                        NoteKind::Touch { .. } => 0.25,
                        _ => 0.1,
                    };
                    *timing = NoteTiming::Dying(Timer::from_seconds(dying_secs, TimerMode::Once));
                }
            }
            NoteTiming::Waiting(timer) => {
                timer.tick(time.delta());
                wait_slide(timer.fraction(), children, &mut slide_elements);
                if timer.just_finished() {
                    // Musical timing → follows the play tempo (/ chart_speed).
                    let total = slide_path_data
                        .map(slide_path::trace_total_secs)
                        .unwrap_or(0.0)
                        / chart_speed;
                    *timing = NoteTiming::Sliding(Timer::from_seconds(total, TimerMode::Once));
                }
            }
            NoteTiming::Sliding(timer) => {
                timer.tick(time.delta());
                if let Some(fan) = fan_lanes {
                    fan_trace(
                        timer.fraction(),
                        fan,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut commands,
                    );
                } else if let Some(path) = slide_path_data {
                    slide_trace(
                        timer.fraction(),
                        path,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut commands,
                    );
                }
                if timer.just_finished() {
                    // Slides have no death visual (shapeless container), so Dying
                    // just despawns children + self on its first frame.
                    *timing = NoteTiming::Dying(Timer::from_seconds(0.0, TimerMode::Once));
                }
            }
            NoteTiming::Dying(timer) => {
                let is_touch = matches!(kind, NoteKind::Touch { .. });

                // On the very first frame: morph into the death effect.
                if timer.elapsed().is_zero() && !is_slide(kind) {
                    if !matches!(kind, NoteKind::Touch { .. }) {
                        guide_sound_messages.write(PlayGuideSoundMessage);
                    }
                    // Despawn all children (slide stars, hold bodies, touch triangles).
                    if let Some(children) = children {
                        for child in children.iter() {
                            commands.entity(child).despawn();
                        }
                    }
                    if is_touch {
                        // Hide the centre sprite; the burst plays out as child entities.
                        if let Some(sprite) = sprite.as_deref_mut() {
                            set_alpha(sprite, 0.0);
                        }
                        commands
                            .entity(entity)
                            .with_children(|p| spawn_touch_spark(p, &assets));
                    } else if let Some(sprite) = sprite.as_deref_mut() {
                        // Tap / SlideStar carry their own sprite: swap it in place for
                        // the hexagon pop effect (alpha-animated below this frame).
                        *sprite = hexagon_sprite(&assets);
                    } else if matches!(kind, NoteKind::TouchHold { .. } | NoteKind::TapHold { .. })
                    {
                        // Hold parents have no sprite of their own — attach the pop.
                        commands.entity(entity).insert(hexagon_sprite(&assets));
                    }
                }

                timer.tick(time.delta());
                let t = timer.fraction();

                if is_touch {
                    // Burst children are spawned via command, so `children` only
                    // includes them from the second frame onward.
                    animate_touch_spark(t, children, &mut touch_sparks);
                } else {
                    // Pop: scale 1.0 -> 2.0 -> 1.0, alpha 0% -> 100% -> 0%.
                    let wave = (t * std::f32::consts::PI).sin();
                    transform.scale = Vec3::splat(1.0 + wave);
                    if let Some(sprite) = sprite.as_deref_mut() {
                        set_alpha(sprite, wave);
                    }
                }

                if timer.just_finished() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}
