use super::{
    NOTE_RADIUS,
    component::{
        FanLanes, HoldNoteElement, NoteBpm, NoteHalo, NoteTiming, SlideArrow, SlideElement,
        TouchElement, TouchHoldCountdown, TouchSpark,
    },
    note_colors,
    resources::{ButtonLayout, NoteAssets},
    shapes::{
        CHEVRON_ROT_OFFSET, COUNTDOWN_EDGE_COLORS, COUNTDOWN_EDGES, COUNTDOWN_SWEEP_S,
        NORMAL_HALO_DOT_OFFSET, chevron_shape, chevron_sprite, countdown_edge, hold_beam_sprite,
        hold_cap_sprite, hold_end_dot_sprite, normal_halo_sprite, star_break_sprite,
        star_ex_sprite, star_sprite, tap_break_sprite, tap_ex_sprite, tap_sprite,
        touch_circle_sprite, touch_effect_sprite, touch_effect_star_sprite,
        touch_hold_triangle_sprite, touch_triangle_sprite, touch_triangle_start_distance,
    },
    slide_path,
};
use crate::systems::{
    GROWING,
    chart_playback::ChartPlayback,
    component::{ChartEvent, Note, NoteKind, SlideShape},
};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use bevy::{ecs::hierarchy::ChildOf, sprite::Anchor};
use bevy_kira_audio::prelude::*;
use bevy_prototype_lyon::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

// ── Main system ────────────────────────────────────────────────────────────

pub fn next_event(
    mut commands: Commands,
    mut chart: ResMut<ChartPlayback>,
    note_assets: Res<NoteAssets>,
    layout: Res<ButtonLayout>,
) {
    if !chart.is_playing() {
        return;
    }

    // `chart.elapsed_time` is maintained by `tick_clock`: slaved to the BGM's
    // playback position when a song is present (drift-free), or frame-driven when
    // the chart plays silently. Either way spawning just reads `elapsed_time`.
    //
    // .map() clones event data and releases the &mut borrow on chart,
    // so chart.chart_speed() / chart.note_speed() are accessible in the loop body.
    while let Some((event, bpm)) = chart.advance().map(|e| (e.event.clone(), e.bpm)) {
        if let ChartEvent::NoteGroup(notes) = event {
            let is_paired = notes.len() >= 2;
            for note in &notes {
                spawn_note(
                    &mut commands,
                    note,
                    is_paired,
                    bpm,
                    chart.chart_speed(),
                    chart.note_speed(),
                    &note_assets,
                    &layout,
                );
            }
        }
    }

    if chart.is_finished() {
        chart.pause();
    }
}

// ── Transport commands ──────────────────────────────────────────────────────

/// Drain JS transport intents (pause/resume/restart/speed) and apply them to the
/// BGM channel + chart clock. The BGM is the clock's source of truth (see
/// `next_event`), so pausing the channel freezes the chart, and seeking the BGM
/// to 0 rewinds it — we just clear the spawned notes and reset the spawn index.
pub fn apply_commands(
    mut commands: Commands,
    mut chart: ResMut<ChartPlayback>,
    bgm_channel: Res<AudioChannel<crate::systems::audio::Bgm>>,
    bgm_instance: Option<Res<crate::systems::audio::BgmInstance>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    notes: Query<Entity, With<NoteTiming>>,
) {
    for cmd in crate::wasm_bridge::take_commands() {
        match cmd {
            crate::wasm_bridge::EngineCommand::Pause => {
                bgm_channel.pause();
                chart.pause();
            }
            crate::wasm_bridge::EngineCommand::Resume => {
                bgm_channel.resume();
                chart.resume();
            }
            crate::wasm_bridge::EngineCommand::SetSongSpeed(rate) => {
                bgm_channel.set_playback_rate(rate as f64);
                chart.set_chart_speed(rate);
            }
            crate::wasm_bridge::EngineCommand::SetNoteSpeed(speed) => chart.set_note_speed(speed),
            crate::wasm_bridge::EngineCommand::Restart => {
                // Rewind the audio (the clock anchor) and clear every spawned note.
                if let Some(instance) = bgm_instance
                    .as_ref()
                    .and_then(|h| audio_instances.get_mut(&h.0))
                {
                    instance.seek_to(0.0);
                }
                for entity in &notes {
                    commands.entity(entity).despawn();
                }
                chart.restart();
                bgm_channel.resume();
            }
        }
    }
}

// ── Per-note spawning ──────────────────────────────────────────────────────

fn spawn_note(
    commands: &mut Commands,
    note: &Note,
    is_paired: bool,
    bpm: f32,
    chart_speed: f32,
    note_speed: f32,
    assets: &NoteAssets,
    layout: &ButtonLayout,
) {
    // Travel is faster with both play speed and note speed:
    // duration ∝ 1 / (chart_speed · note_speed).
    let growing_time = GROWING as f32 / (chart_speed * note_speed);

    let mut e = commands.spawn((
        note.kind.clone(),
        NoteTiming::Growing(Timer::from_seconds(growing_time, TimerMode::Once)),
        NoteBpm(bpm),
    ));

    match &note.kind {
        NoteKind::Tap(id) => {
            let pos = tap_pos(*id, layout) * super::RADIUS;
            let dir = layout.tap[*id - 1];
            let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
            let base = if note.is_break {
                tap_break_sprite(assets)
            } else {
                tap_sprite(assets)
            };
            e.insert((
                base,
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
            ));
            e.with_children(|p| {
                spawn_note_halo(p, assets, Quat::from_rotation_z(angle));
                if note.is_ex {
                    spawn_ex_overlay(p, tap_ex_sprite(assets));
                }
            });
        }

        NoteKind::TapHold { button, .. } => {
            let pos = tap_pos(*button, layout) * super::RADIUS;
            let dir = layout.tap[*button - 1];
            let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
            e.insert((
                Visibility::default(),
                Transform::from_translation(pos.extend(2.0))
                    .with_rotation(Quat::from_rotation_z(angle))
                    .with_scale(Vec3::ZERO),
            ));
            e.with_children(|p| spawn_hold_children(p, assets, note.is_break, note.is_ex));
        }

        NoteKind::Touch { value, group } => {
            let pos = touch_pos(*value, *group, layout) * super::RADIUS;
            e.insert((
                touch_circle_sprite(assets),
                Transform::from_translation(pos.extend(2.0)),
                TouchElement::Center,
                Visibility::Hidden,
            ));
            e.with_children(|p| spawn_approach_triangles(p, is_paired, false, assets));
        }

        NoteKind::TouchHold { value, group, .. } => {
            let pos = touch_pos(*value, *group, layout) * super::RADIUS;
            e.insert((
                touch_circle_sprite(assets),
                Transform::from_translation(pos.extend(2.0)),
                TouchElement::Center,
                Visibility::Hidden,
            ));
            e.with_children(|p| {
                spawn_approach_triangles(p, is_paired, true, assets);
                spawn_touch_hold_countdown(p);
            });
        }

        // Standalone star (no path): behaves like a tap, parent carries the star.
        NoteKind::SlideStar(button) => {
            let pos = tap_pos(*button, layout) * super::RADIUS;
            let dir = layout.tap[*button - 1];
            let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
            let base = if note.is_break {
                star_break_sprite(assets)
            } else {
                star_sprite(assets)
            };
            e.insert((
                base,
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
            ));
            e.with_children(|p| {
                spawn_note_halo(p, assets, Quat::from_rotation_z(angle));
                if note.is_ex {
                    spawn_ex_overlay(p, star_ex_sprite(assets));
                }
            });
        }

        // Slide / HeadlessSlide: parent is an origin container; head star, trace
        // star, and chevrons live as children at absolute world positions.
        NoteKind::Slide {
            head_button,
            segments,
            shared_duration,
        }
        | NoteKind::HeadlessSlide {
            start_button: head_button,
            segments,
            shared_duration,
        } => {
            let path = slide_path::build_slide_trace(
                segments,
                *head_button,
                bpm,
                *shared_duration,
                layout,
            );
            let has_head = matches!(note.kind, NoteKind::Slide { .. });
            let head = *head_button;

            // Fan (`w`): a single FanShape segment fans into 3 diverging lanes,
            // each with its own growing chevrons and trace star.
            let fan_ends = match segments.as_slice() {
                [seg] => match seg.shape {
                    SlideShape::FanShape { ends } => Some(ends),
                    _ => None,
                },
                _ => None,
            };

            e.insert((Transform::default(), Visibility::Visible));
            if let Some((e1, e2, e3)) = fan_ends {
                let mut lanes = Vec::with_capacity(3);
                let mut lengths = Vec::with_capacity(3);
                for end in [e1, e2, e3] {
                    let pts =
                        slide_path::generate_points(&SlideShape::Straight { end }, head, layout);
                    lengths.push(slide_path::calculate_total_length(&pts));
                    lanes.push(pts);
                }
                e.with_children(|p| {
                    spawn_fan_children(p, &lanes, &lengths, has_head, head, layout, assets);
                });
                e.insert((path, FanLanes { lanes, lengths }));
            } else {
                e.with_children(|p| {
                    spawn_slide_children(p, &path, has_head, head, layout, assets);
                });
                e.insert(path);
            }
        }
    }
}

// ── Transform helpers ──────────────────────────────────────────────────────

fn tap_pos(value: usize, layout: &ButtonLayout) -> Vec2 {
    layout.tap_spawn[value - 1]
}

fn touch_pos(value: usize, group: char, layout: &ButtonLayout) -> Vec2 {
    match group.to_ascii_uppercase() {
        'C' => layout.c[value - 1],
        'B' => layout.b[value - 1],
        'A' => layout.a[value - 1],
        'D' => layout.d[value - 1],
        'E' => layout.e[value - 1],
        _ => Vec2::ZERO,
    }
}

// ── Child entity spawners ──────────────────────────────────────────────────

/// Faint halo ring (`Normal.png`) drawn behind a tap/star note. The anchor sits
/// on the ring's glow dot (`NORMAL_HALO_DOT_OFFSET` above the texture centre),
/// pinning the dot to the note centre at every scale — the sprite scales about
/// its anchor, so the dot never drifts. The mover rescales the ring every frame
/// so it stays concentric with the judgement circle (see `scale_note_halo`).
///
/// `rotation` must orient the dot's +Y onto the outward radial direction (the
/// outward angle − π/2). Tap/star parents carry no rotation, so they pass it
/// here; a hold parent is already rotated, so its halo passes `Quat::IDENTITY`
/// and inherits the parent's rotation instead.
fn spawn_note_halo(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    assets: &NoteAssets,
    rotation: Quat,
) {
    parent.spawn((
        normal_halo_sprite(assets),
        Anchor(Vec2::new(0.0, NORMAL_HALO_DOT_OFFSET)),
        NoteHalo,
        // z = -0.5 (relative) → behind the note body.
        Transform::from_xyz(0.0, 0.0, -0.5).with_rotation(rotation),
    ));
}

/// EX outline overlay, drawn just in front of the note body. Inherits the
/// parent's grow/move transform and is despawned with the note.
fn spawn_ex_overlay(parent: &mut RelatedSpawnerCommands<ChildOf>, ex: Sprite) {
    parent.spawn((ex, Transform::from_xyz(0.0, 0.0, 0.1)));
}

/// Spawn one head/body/tail slice triple at depth `z`. The mover animates every
/// `HoldNoteElement` child by its marker, so base and EX-overlay triples both
/// stretch/track correctly.
fn spawn_hold_slices(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    head: Handle<Image>,
    body: Handle<Image>,
    tail: Handle<Image>,
    z: f32,
) {
    parent.spawn((
        hold_cap_sprite(head),
        Anchor::BOTTOM_CENTER,
        Transform::from_xyz(0.0, 0.0, z),
        HoldNoteElement::Head,
    ));
    parent.spawn((
        hold_beam_sprite(body),
        Transform::from_xyz(0.0, -0.001, z).with_scale(Vec3::new(1.0, 0.001, 1.0)),
        HoldNoteElement::Body,
    ));
    parent.spawn((
        hold_cap_sprite(tail),
        Anchor::TOP_CENTER,
        Transform::from_xyz(0.0, -0.001, z),
        HoldNoteElement::Tail,
    ));
}

fn spawn_hold_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    assets: &NoteAssets,
    is_break: bool,
    is_ex: bool,
) {
    // Faint concentric halo behind the head. The hold parent already carries the
    // radial rotation, so the halo inherits it (identity rotation here).
    spawn_note_halo(parent, assets, Quat::IDENTITY);

    // break → swap the base slices for the break variant.
    let (head, body, tail) = if is_break {
        (
            &assets.hold_break_head,
            &assets.hold_break_body,
            &assets.hold_break_tail,
        )
    } else {
        (&assets.hold_head, &assets.hold_body, &assets.hold_tail)
    };
    spawn_hold_slices(parent, head.clone(), body.clone(), tail.clone(), 0.0);

    // ex → overlay the EX outline slices on top (same markers ⇒ same motion).
    if is_ex {
        spawn_hold_slices(
            parent,
            assets.hold_ex_head.clone(),
            assets.hold_ex_body.clone(),
            assets.hold_ex_tail.clone(),
            0.1,
        );
    }

    // Glow dot pinned to the tail end (mover drives its y to -len).
    parent.spawn((
        hold_end_dot_sprite(assets),
        Transform::from_xyz(0.0, -0.001, 0.2),
        HoldNoteElement::EndDot,
    ));
}

fn spawn_touch_hold_countdown(parent: &mut RelatedSpawnerCommands<ChildOf>) {
    // The coloured diamond border, one entity per edge. Each starts empty and is
    // drawn in clockwise (revealed by `hold_touch` as the countdown progresses).
    for edge in 0..COUNTDOWN_EDGES {
        parent.spawn((
            ShapeBuilder::with(&countdown_edge(edge, COUNTDOWN_SWEEP_S, 0.0))
                .stroke((COUNTDOWN_EDGE_COLORS[edge], NOTE_RADIUS * 0.18))
                .build(),
            Transform::from_xyz(0.0, 0.0, 5.0),
            Visibility::Hidden,
            TouchHoldCountdown { edge },
        ));
    }
}

/// Touch death burst: an expanding halo, 8 stars that converge inward, and 4
/// stars that burst outward. All animated from the parent's `Dying` timer in
/// `animate_touch_spark`.
pub(super) fn spawn_touch_spark(parent: &mut RelatedSpawnerCommands<ChildOf>, assets: &NoteAssets) {
    // Halo: expanding, fading burst centred on the note (TouchEff.png).
    parent.spawn((
        touch_effect_sprite(assets),
        Transform::from_xyz(0.0, 0.0, 1.0),
        TouchSpark::Halo,
    ));

    // 8 burst stars converging inward, evenly spaced; parts alternate 01/02.
    for i in 0..8 {
        let angle = i as f32 * (PI / 4.0);
        let pos = Vec2::new(angle.cos(), angle.sin()) * super::SPARK_STAR_RADIUS;
        parent.spawn((
            touch_effect_star_sprite(assets, i),
            Transform::from_translation(pos.extend(2.0))
                .with_scale(Vec3::splat(super::SPARK_STAR_SCALE)),
            TouchSpark::StarIn(angle),
        ));
    }

    // 8 burst stars bursting outward, offset 22.5° to emerge between the converged points.
    for i in 0..8 {
        let angle = PI / 8.0 + i as f32 * (PI / 4.0);
        parent.spawn((
            touch_effect_star_sprite(assets, i),
            Transform::from_translation(Vec3::new(0.0, 0.0, 2.0))
                .with_scale(Vec3::splat(super::SPARK_STAR_SCALE)),
            TouchSpark::StarOut(angle),
        ));
    }
}

fn spawn_approach_triangles(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    is_paired: bool,
    is_hold: bool,
    assets: &NoteAssets,
) {
    let dist = touch_triangle_start_distance(NOTE_RADIUS);

    // Sprites point +Y; rotating by `atan2(dir) - π/2` aims each at its outward
    // direction. Regular Touch: 4 cardinal triangles share `Touch_01.png`.
    if !is_hold {
        let _ = is_paired;
        for dir in [Vec2::NEG_X, Vec2::X, Vec2::Y, Vec2::NEG_Y] {
            parent.spawn((
                touch_triangle_sprite(assets),
                bevy::sprite::Anchor(Vec2::new(0.0, 0.3)),
                Transform::from_translation((dir * dist).extend(-0.1))
                    .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x) + FRAC_PI_2)),
                TouchElement::Triangle,
            ));
        }
    } else {
        // TouchHold: 4 diagonal triangles, one coloured sprite each.
        let dirs = [
            Vec2::ONE.normalize(),
            Vec2::new(1.0, -1.0).normalize(),
            Vec2::NEG_ONE.normalize(),
            Vec2::new(-1.0, 1.0).normalize(),
        ];
        for (idx, dir) in dirs.iter().enumerate() {
            parent.spawn((
                touch_hold_triangle_sprite(assets, idx),
                bevy::sprite::Anchor(Vec2::new(0.0, 0.3)),
                Transform::from_translation((dir * dist).extend(-0.1))
                    .with_rotation(Quat::from_rotation_z(dir.y.atan2(dir.x) + FRAC_PI_2)),
                TouchElement::Triangle,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_slide_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    path: &super::component::SlidePath,
    has_head: bool,
    head_button: usize,
    layout: &ButtonLayout,
    assets: &NoteAssets,
) {
    // Head star (Slide only): spawns at the head button, scale ZERO so it grows
    // during the Growing phase, then approaches the ring during Moving. Its
    // concentric halo rides along as a child (the head star carries no rotation,
    // so the halo supplies the radial rotation itself, like a tap).
    if has_head {
        let pos = layout.tap_spawn[head_button - 1] * super::RADIUS;
        let dir = layout.tap[head_button - 1];
        let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
        parent
            .spawn((
                star_sprite(assets),
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
                SlideElement::Head,
            ))
            .with_children(|h| spawn_note_halo(h, assets, Quat::from_rotation_z(angle)));
    }

    // Trace star: hidden at the path start; revealed and faded in during Waiting,
    // then moved along the path during Sliding.
    let start = path.waypoints.first().copied().unwrap_or(Vec2::ZERO);
    parent.spawn((
        star_sprite(assets),
        Transform::from_translation(start.extend(3.0)),
        Visibility::Hidden,
        SlideElement::TraceStar(0),
    ));

    // Chevron arrows of constant size along the path (sprite).
    spawn_chevron_line(
        parent,
        &path.waypoints,
        path.total_length,
        0,
        super::CHEVRON_SPACING,
        assets,
        false,
        |_| NOTE_RADIUS,
    );
}

/// Place chevrons along `lane` every `spacing` units, each rotated to the travel
/// direction and spawned transparent (faded in during Growing; consumed during
/// Sliding). `radius_at(f)` gives the chevron wing-radius at fraction `f∈[0,1)`
/// along the lane — constant for ordinary slides, growing for the fan cone.
#[allow(clippy::too_many_arguments)]
fn spawn_chevron_line(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    lane: &[Vec2],
    length: f32,
    lane_idx: usize,
    spacing: f32,
    assets: &NoteAssets,
    // Fan / wifi cones use the lyon vector chevron (wings scale, stem fixed);
    // ordinary slides use the sprite chevron.
    lyon: bool,
    radius_at: impl Fn(f32) -> f32,
) {
    let mut d = spacing;
    while d < length {
        let (pos, angle) = slide_path::get_transform_at_distance(lane, d);
        let radius = radius_at(d / length);
        let arrow = SlideArrow {
            distance_along_path: d,
            lane: lane_idx,
        };
        if lyon {
            // Lyon chevron tip points +X → rotate by the bare travel angle.
            parent.spawn((
                chevron_shape(
                    radius,
                    0.5 * NOTE_RADIUS,
                    note_colors::CHEVRON.with_alpha(0.0),
                ),
                Transform::from_translation(pos.extend(1.0))
                    .with_rotation(Quat::from_rotation_z(angle)),
                arrow,
            ));
        } else {
            parent.spawn((
                chevron_sprite(assets, radius, 0.5 * NOTE_RADIUS),
                Transform::from_translation(pos.extend(1.0))
                    .with_rotation(Quat::from_rotation_z(angle + CHEVRON_ROT_OFFSET)),
                arrow,
            ));
        }
        d += spacing;
    }
}

/// Fan (`w`) variant: one head star plus, per lane, a trace star and a line of
/// chevrons whose size grows with distance (small near the start, large near
/// the end), all diverging to the three end buttons.
#[allow(clippy::too_many_arguments)]
fn spawn_fan_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    lanes: &[Vec<Vec2>],
    lengths: &[f32],
    has_head: bool,
    head_button: usize,
    layout: &ButtonLayout,
    assets: &NoteAssets,
) {
    if has_head {
        let pos = layout.tap_spawn[head_button - 1] * super::RADIUS;
        let dir = layout.tap[head_button - 1];
        let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
        parent
            .spawn((
                star_sprite(assets),
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
                SlideElement::Head,
            ))
            .with_children(|h| spawn_note_halo(h, assets, Quat::from_rotation_z(angle)));
    }

    // One trace star per lane (diverges to its end during Sliding).
    for (idx, lane) in lanes.iter().enumerate() {
        let start = lane.first().copied().unwrap_or(Vec2::ZERO);
        parent.spawn((
            star_sprite(assets),
            Transform::from_translation(start.extend(3.0)),
            Visibility::Hidden,
            SlideElement::TraceStar(idx),
        ));
    }

    // A single widening cone of chevrons along the central lane (→ e, the
    // bisector of the two neighbours): wing-radius grows base → max with
    // distance while thickness/angle stay fixed. Consumes against lane 0.
    let base_radius = 0.8 * NOTE_RADIUS;
    let max_radius = 7.5 * NOTE_RADIUS;
    spawn_chevron_line(
        parent,
        &lanes[0],
        lengths[0],
        0,
        1.5 * super::CHEVRON_SPACING,
        assets,
        true, // fan cone → lyon vector chevrons
        |f| base_radius + f * (max_radius - base_radius),
    );
}
