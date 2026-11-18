use super::majdata::{SlideLayouts, sprite_px};
use super::schedule::phase_at;
use super::{
    NOTE_RADIUS,
    component::{
        HitTime, HoldNoteElement, NoteBpm, NoteHalo, NoteTiming, SlideArrow, SlideElement,
        TouchElement, TouchHoldCountdown, TouchSpark,
    },
    resources::{ButtonLayout, NoteAssets},
    shapes::{
        COUNTDOWN_EDGE_COLORS, COUNTDOWN_SWEEP_S, NORMAL_HALO_DOT_OFFSET, countdown_edge,
        hold_beam_sprite, hold_cap_sprite, hold_end_dot_sprite, normal_halo_sprite,
        star_break_sprite, star_ex_sprite, star_sprite, tap_break_sprite, tap_ex_sprite,
        tap_sprite, touch_circle_sprite, touch_effect_sprite, touch_effect_star_sprite,
        touch_hold_triangle_sprite, touch_triangle_sprite, touch_triangle_start_distance,
    },
    slide_path,
};
use crate::systems::{
    chart_playback::ChartPlayback,
    component::{ChartEvent, Note, NoteKind},
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
    layouts: Res<SlideLayouts>,
    on_screen: Query<Entity, With<NoteTiming>>,
) {
    let mut spawned = false;

    // A seek (visualizer-majdata 08): replace whatever is on screen with the
    // notes live at the new time, playing or paused, so a paused seek shows
    // the chart there. Spawning then carries on after them (`seek`).
    if let Some(t) = chart.take_respawn() {
        // `try_`: a load or an unload in the same frame may have despawned
        // them already.
        for entity in &on_screen {
            commands.entity(entity).try_despawn();
        }
        let note_speed = chart.note_speed();
        for event in chart.live_events(t) {
            let ChartEvent::NoteGroup(notes) = &event.event else {
                continue;
            };
            for note in notes {
                if phase_at(&note.kind, event.time, event.bpm, note_speed, t).is_some() {
                    spawned = true;
                    spawn_note(
                        &mut commands,
                        note,
                        is_paired(note, notes),
                        event.bpm,
                        event.time,
                        &note_assets,
                        &layout,
                        &layouts,
                    );
                }
            }
        }
    }

    if !chart.is_playing() {
        return;
    }

    // `chart.elapsed_time` is maintained by `tick_clock`: slaved to the BGM's
    // playback position when a song is present (drift-free), or frame-driven when
    // the chart plays silently. Either way spawning just reads `elapsed_time`.
    //
    // .map() clones event data and releases the &mut borrow on chart,
    // so chart.chart_speed() / chart.note_speed() are accessible in the loop body.
    while let Some((event, bpm, hit)) = chart.advance().map(|e| (e.event.clone(), e.bpm, e.time)) {
        if let ChartEvent::NoteGroup(notes) = event {
            spawned = true;
            for note in &notes {
                spawn_note(
                    &mut commands,
                    note,
                    is_paired(note, &notes),
                    bpm,
                    hit,
                    &note_assets,
                    &layout,
                    &layouts,
                );
            }
        }
    }

    // The chart is over once everything is spawned and the last note has
    // played out. Stopping when the last note *spawns* froze it mid-approach,
    // since a paused chart freezes every note. Notes spawned this frame are
    // not in `on_screen` yet (deferred commands), hence `spawned`.
    if chart.all_notes_spawned() && !spawned && on_screen.is_empty() {
        chart.pause();
    }
}

/// EACH-ness is per sub-comma offset, not per token. Notes split by a
/// pseudo-EACH backtick are 1ms apart, so the notation is explicit that they
/// are *not* an EACH and must not render as a pair. Groups are at most a
/// handful of notes, so the scan is cheaper than building a map.
fn is_paired(note: &Note, group: &[Note]) -> bool {
    group
        .iter()
        .filter(|other| other.offset_ms == note.offset_ms)
        .count()
        >= 2
}

// ── Transport commands ──────────────────────────────────────────────────────

/// Drain JS transport intents (pause/resume/restart/speed) and apply them to the
/// BGM channel + chart clock. The BGM is the clock's source of truth (see
/// `next_event`), so pausing the channel freezes the chart, and seeking the BGM
/// to 0 rewinds it — we just clear the spawned notes and reset the spawn index.
#[allow(clippy::too_many_arguments)]
pub fn apply_commands(
    mut commands: Commands,
    mut chart: ResMut<ChartPlayback>,
    bgm_channel: Res<AudioChannel<crate::systems::audio::Bgm>>,
    bgm_instance: Option<Res<crate::systems::audio::BgmInstance>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    notes: Query<Entity, With<NoteTiming>>,
    camera: Query<Entity, With<crate::plugins::camera::MainCamera>>,
    mut window: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut audio_lead: ResMut<crate::systems::audio::AudioLead>,
    mut guide_scheduler: ResMut<crate::systems::audio::GuideScheduler>,
    mut metronome: ResMut<crate::systems::audio::Metronome>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    for cmd in crate::wasm_bridge::take_commands() {
        match cmd {
            crate::wasm_bridge::EngineCommand::Load(song) => crate::systems::ingest_song(
                song,
                &mut commands,
                &mut chart,
                &mut guide_scheduler,
                &mut audio_sources,
                &notes,
            ),
            crate::wasm_bridge::EngineCommand::SetAudioLeadMs(ms) => audio_lead.wall_ms = ms,
            // Diagnostics (mobile-webview-spike 02). Msaa is a camera
            // component; inserting replaces the default Sample4.
            crate::wasm_bridge::EngineCommand::SetMsaa(samples) => {
                if let Ok(cam) = camera.single() {
                    commands.entity(cam).insert(Msaa::from_samples(samples));
                }
            }
            crate::wasm_bridge::EngineCommand::SetRenderScale(scale) => {
                if let Ok(mut win) = window.single_mut() {
                    win.resolution.set_scale_factor_override(scale);
                }
            }
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
            crate::wasm_bridge::EngineCommand::Seek(secs) => {
                // The clock jumps; `next_event` swaps the notes on screen. The
                // music follows (its position is chart time), and the guide
                // sounds and clicks restart there without sounding the gap.
                chart.seek(secs);
                let t = chart.elapsed_time();
                if let Some(instance) = bgm_instance
                    .as_ref()
                    .and_then(|h| audio_instances.get_mut(&h.0))
                {
                    instance.seek_to(t);
                }
                let speed = chart.chart_speed();
                let horizon = t + audio_lead.chart_secs(speed);
                guide_scheduler.seek(chart.timed_events(), t);
                metronome.seek(chart.timed_events(), horizon, speed);
            }
            crate::wasm_bridge::EngineCommand::Unload => {
                // Leaving the Play page: nothing of this chart may outlive it.
                for entity in &notes {
                    commands.entity(entity).try_despawn();
                }
                chart.unload();
                crate::wasm_bridge::bump_chart(String::new());
                guide_scheduler.reset();
                bgm_channel.stop();
                commands.remove_resource::<crate::systems::audio::BgmInstance>();
                commands.remove_resource::<crate::systems::audio::BgmSource>();
            }
            crate::wasm_bridge::EngineCommand::Restart => {
                // Rewind the audio (the clock anchor) and clear every spawned note.
                if let Some(instance) = bgm_instance
                    .as_ref()
                    .and_then(|h| audio_instances.get_mut(&h.0))
                {
                    instance.seek_to(0.0);
                }
                for entity in &notes {
                    commands.entity(entity).try_despawn();
                }
                chart.restart();
                bgm_channel.resume();
            }
        }
    }
}

// ── Per-note spawning ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn spawn_note(
    commands: &mut Commands,
    note: &Note,
    is_paired: bool,
    bpm: f32,
    hit: f64,
    assets: &NoteAssets,
    layout: &ButtonLayout,
    layouts: &SlideLayouts,
) {
    // The timer is a placeholder: `update_movement` sets the phase from the
    // chart clock on the note's first frame.
    let mut e = commands.spawn((
        note.kind.clone(),
        NoteTiming::Growing(Timer::default()),
        NoteBpm(bpm),
        HitTime(hit),
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
        // `rotates` (`$$`) is not drawn yet — both spellings get the same sprite.
        NoteKind::SlideStar { button, .. } => {
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
        // A slide's head star, trace star(s) and arrows live as children at
        // absolute world positions; the arrows are MajdataPlay's, from its
        // slide layouts (visualizer-majdata 14).
        NoteKind::Slide {
            head_button,
            segments,
            shared_duration,
            ..
        }
        | NoteKind::HeadlessSlide {
            start_button: head_button,
            segments,
            shared_duration,
        } => {
            let built = slide_path::build_slide_from_layout(
                segments,
                *head_button,
                bpm,
                *shared_duration,
                layouts,
                layout,
            );
            // `None`: headless. `Some(sprite)`: the head, drawn as the automatic
            // star or — under the `@` modifier — as a normal tap.
            let head_sprite = match note.kind {
                NoteKind::Slide {
                    plain_head: true, ..
                } => Some(tap_sprite(assets)),
                NoteKind::Slide { .. } => Some(star_sprite(assets)),
                _ => None,
            };
            let art = if note.is_break {
                ArrowArt::Break
            } else if is_paired {
                ArrowArt::Each
            } else {
                ArrowArt::Normal
            };
            e.insert((Transform::default(), Visibility::Visible));
            e.with_children(|p| {
                spawn_slide_children(p, &built, head_sprite, *head_button, art, layout, assets);
            });
            let slide_path::BuiltSlide { path, fan, .. } = built;
            e.insert(path);
            if let Some(fan) = fan {
                e.insert(fan);
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
    for (edge, color) in COUNTDOWN_EDGE_COLORS.iter().enumerate() {
        parent.spawn((
            ShapeBuilder::with(&countdown_edge(edge, COUNTDOWN_SWEEP_S, 0.0))
                .stroke((*color, NOTE_RADIUS * 0.18))
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

/// MajdataPlay's arrow art: normal, each, break.
const SLIDE_PATHS: [&str; 3] = [
    "skin/SlideSkins/slide.png",
    "skin/SlideSkins/slide_each.png",
    "skin/SlideSkins/slide_break.png",
];
/// A fan's rows, each with its own image.
pub(super) const WIFI_ROWS: usize = 11;

fn wifi_path(art: ArrowArt, row: usize) -> String {
    let variant = ["", "each_", "break_"][art as usize];
    format!("skin/WifiSkins/wifi_{variant}{row}.png")
}

/// Which MajdataPlay arrow art a slide uses.
#[derive(Clone, Copy)]
enum ArrowArt {
    Normal = 0,
    Each = 1,
    Break = 2,
}

#[allow(clippy::too_many_arguments)]
fn spawn_slide_children(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    built: &slide_path::BuiltSlide,
    head_sprite: Option<Sprite>,
    head_button: usize,
    art: ArrowArt,
    layout: &ButtonLayout,
    assets: &NoteAssets,
) {
    // Head (Slide only; `None` for a HeadlessSlide): spawns at the head button,
    // scale ZERO so it grows during the Growing phase, then approaches the ring
    // during Moving. Its concentric halo rides along as a child (the head
    // carries no rotation, so the halo supplies the radial rotation itself,
    // like a tap). The sprite is the star, or a plain tap under `@`.
    if let Some(sprite) = head_sprite {
        let pos = layout.tap_spawn[head_button - 1] * super::RADIUS;
        let dir = layout.tap[head_button - 1];
        let angle = dir.y.atan2(dir.x) - FRAC_PI_2;
        parent
            .spawn((
                sprite,
                Transform::from_translation(pos.extend(2.0)).with_scale(Vec3::ZERO),
                SlideElement::Head,
            ))
            .with_children(|h| spawn_note_halo(h, assets, Quat::from_rotation_z(angle)));
    }

    // Trace star(s): hidden at the path start; revealed and faded in during
    // Waiting, then moved along the path (a fan: one per lane) during Sliding.
    let starts: Vec<Vec2> = match &built.fan {
        Some(fan) => fan.lanes.iter().map(|l| l[0]).collect(),
        None => vec![built.path.waypoints.first().copied().unwrap_or(Vec2::ZERO)],
    };
    for (lane, start) in starts.into_iter().enumerate() {
        parent.spawn((
            star_sprite(assets),
            Transform::from_translation(start.extend(3.0)),
            Visibility::Hidden,
            SlideElement::TraceStar(lane),
        ));
    }

    // MajdataPlay's arrows (a fan: its rows), transparent until Growing fades
    // them in; each clears as the star passes it.
    for arrow in &built.arrows {
        let (image, path) = if built.fan.is_some() {
            let row = arrow.row.min(WIFI_ROWS - 1);
            (assets.wifi[art as usize][row].clone(), wifi_path(art, row))
        } else {
            (
                assets.slide[art as usize].clone(),
                SLIDE_PATHS[art as usize].to_string(),
            )
        };
        let mut sprite = Sprite {
            image,
            custom_size: Some(sprite_px(&path)),
            ..default()
        };
        sprite.color.set_alpha(0.0);
        parent.spawn((
            sprite,
            Transform::from_translation(arrow.pos.extend(1.0))
                .with_rotation(Quat::from_rotation_z(arrow.angle)),
            SlideArrow {
                distance_along_path: arrow.distance,
                lane: 0,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::visual::component::NoteTiming;

    fn app_with_chart(chart: &str) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<NoteAssets>()
            .init_resource::<ButtonLayout>()
            .init_resource::<crate::systems::visual::majdata::SlideLayouts>()
            .init_resource::<ChartPlayback>()
            .add_systems(Update, next_event);
        app.world_mut()
            .resource_mut::<ChartPlayback>()
            .load_chart(simai::parse_chart(chart).unwrap());
        app
    }

    /// The chart runs until its last note has played out, not until that note
    /// spawns: a paused chart freezes every note, so stopping at spawn froze
    /// the last notes mid-approach (found planning visualizer-majdata piece 2).
    #[test]
    fn the_chart_keeps_running_while_its_last_note_is_on_screen() {
        let mut app = app_with_chart("(120){4}1,E");
        app.update();
        app.update();
        let notes = app
            .world_mut()
            .query_filtered::<(), With<NoteTiming>>()
            .iter(app.world())
            .count();
        assert_eq!(notes, 1, "the only note spawned");
        assert!(app.world().resource::<ChartPlayback>().is_playing());
    }

    #[test]
    fn the_chart_stops_once_its_last_note_is_gone() {
        let mut app = app_with_chart("(120){4}1,E");
        app.update();
        let spawned: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NoteTiming>>()
            .iter(app.world())
            .collect();
        for e in spawned {
            app.world_mut().despawn(e);
        }
        app.update();
        assert!(!app.world().resource::<ChartPlayback>().is_playing());
    }

    fn hits(app: &mut App) -> Vec<f64> {
        let mut hits: Vec<f64> = app
            .world_mut()
            .query::<&HitTime>()
            .iter(app.world())
            .map(|h| h.0)
            .collect();
        hits.sort_by(f64::total_cmp);
        hits
    }

    /// Review Focus 1, 2 (piece 2): a seek while paused shows the chart at
    /// that time — here a hold half-way through, and not the tap before it.
    #[test]
    fn a_seek_spawns_what_is_on_screen_at_that_time() {
        let mut app = app_with_chart("(120){4}1h[1:1],2,E");
        {
            let mut chart = app.world_mut().resource_mut::<ChartPlayback>();
            chart.pause();
            chart.seek(1.0);
        }
        app.update();
        assert_eq!(hits(&mut app), vec![0.0]);
    }

    /// Review Focus 3 (piece 2).
    #[test]
    fn seeking_twice_leaves_only_the_second_frame() {
        let mut app = app_with_chart("(120){4}1h[1:1],2,E");
        app.world_mut().resource_mut::<ChartPlayback>().pause();
        app.world_mut().resource_mut::<ChartPlayback>().seek(1.0);
        app.update();
        app.world_mut().resource_mut::<ChartPlayback>().seek(0.2);
        app.update();
        assert_eq!(hits(&mut app), vec![0.0, 0.5]);
    }

    /// Review Focus 5 (piece 2): rests after the last note do not keep the
    /// chart running past its length, where the timeline ends.
    #[test]
    fn the_chart_stops_at_its_length_despite_trailing_rests() {
        let mut app = app_with_chart("(120){4}1,,,,,,,,E");
        app.world_mut().resource_mut::<ChartPlayback>().seek(0.3);
        app.update();
        app.update();
        assert!(!app.world().resource::<ChartPlayback>().is_playing());
    }

    /// Final review, piece 2: raising the note speed shortens the approach,
    /// so notes already spawned can fall out of view — and must come back
    /// when they are due, not be lost behind the spawn cursor.
    #[test]
    fn a_faster_note_speed_does_not_lose_notes() {
        let mut app = app_with_chart("(120){16}1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,E");
        app.add_systems(
            Update,
            crate::systems::visual::update_movement.after(next_event),
        );
        {
            let mut chart = app.world_mut().resource_mut::<ChartPlayback>();
            chart.set_note_speed(3.0);
            chart.pause();
            chart.seek(0.0);
        }
        app.update();
        app.world_mut()
            .resource_mut::<ChartPlayback>()
            .set_note_speed(9.0);
        app.update();
        {
            let mut chart = app.world_mut().resource_mut::<ChartPlayback>();
            chart.tick(0.8);
            chart.resume();
        }
        app.update();
        app.update();
        let on_screen = hits(&mut app);
        for due in [0.875, 1.0, 1.125] {
            assert!(on_screen.contains(&due), "{due} missing from {on_screen:?}");
        }
    }

    #[test]
    fn a_slide_has_one_arrow_per_prefab_bar() {
        let mut app = app_with_chart("(120){4}1-5[8:1],E");
        app.update();
        app.update();
        let arrows = app
            .world_mut()
            .query::<&SlideArrow>()
            .iter(app.world())
            .count();
        let bars = crate::systems::visual::majdata::SlideLayouts::load()
            .get("Star_Line_5")
            .unwrap()
            .len();
        assert_eq!(arrows, bars);
    }
}
