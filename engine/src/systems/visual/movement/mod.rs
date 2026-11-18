//! Note lifecycle driver. `update_movement` runs every frame and advances each
//! note through its `NoteTiming` phases (Growing → Moving → Holding/Waiting →
//! Sliding → Dying), delegating per-element animation to [`animation`].

use super::spawning::spawn_touch_spark;
use super::{RADIUS, component::NoteTiming, resources::ButtonLayout, resources::NoteAssets};
use crate::systems::{
    chart_playback::ChartPlayback,
    component::NoteKind,
    visual::{
        component::{
            FanLanes, HitTime, HoldHalo, HoldNoteElement, NoteBpm, NoteHalo, SlideArrow,
            SlideElement, SlidePath, TouchElement, TouchHoldCountdown, TouchSpark,
        },
        schedule::{Phase, TAP_DYING, phase_at},
        shapes::{hexagon_sprite, hold_halo_sprite, touch_just_sprite},
    },
};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

mod animation;
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

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
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
        &HitTime,
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
    assets: Res<NoteAssets>,
) {
    // Every note is placed from the chart clock (visualizer-majdata 06):
    // paused, the clock stands and so does everything on screen; a seek is a
    // jump of the clock.
    let note_speed = chart.note_speed();
    let now_secs = chart.elapsed_time();

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
        hit,
    ) in entity_query.iter_mut()
    {
        let bpm = note_bpm.map(|b| b.0).unwrap_or(0.0);
        let Some(now) = phase_at(kind, hit.0, bpm, note_speed, now_secs) else {
            commands.entity(entity).despawn();
            continue;
        };

        // Entering a later phase runs the entry work of every phase crossed,
        // in order: one per frame in play, several at once after a seek.
        let current = timing.phase();
        for phase in [Phase::Moving, Phase::Holding, Phase::Waiting, Phase::Dying] {
            if phase <= current || phase > now.phase {
                continue;
            }
            match phase {
                Phase::Moving => {
                    if matches!(kind, NoteKind::Touch { .. } | NoteKind::TouchHold { .. }) {
                        *visibility = Visibility::Visible;
                    }
                }
                // Skipped on the way to Dying: Dying's despawn sees only the
                // children that already exist, and the halo would outlive it.
                Phase::Holding if now.phase != Phase::Dying => match kind {
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
                        commands.entity(entity).with_children(|parent| {
                            parent.spawn((
                                touch_just_sprite(&assets),
                                Transform::from_xyz(0.0, 0.0, 0.5),
                            ));
                        });
                    }
                    _ => {}
                },
                Phase::Waiting if is_slide(kind) => {
                    hide_slide_head(children, &mut slide_elements);
                    // Pop effect where the head star lands on the ring: a tap
                    // hit with the slide, so the clock runs it as a pop. Not
                    // re-created by a seek that lands after it ended.
                    if let NoteKind::Slide { head_button, .. } = kind
                        && now_secs - hit.0 < TAP_DYING
                    {
                        let pos = layout.tap[head_button - 1] * RADIUS;
                        // Invisible until its first Dying frame places it:
                        // spawned by a command, it is drawn once before.
                        let mut pop = hexagon_sprite(&assets);
                        set_alpha(&mut pop, 0.0);
                        commands.spawn((
                            pop,
                            Transform::from_translation(pos.extend(2.5)),
                            Visibility::Visible,
                            NoteKind::Tap(*head_button),
                            NoteTiming::Growing(Timer::default()),
                            HitTime(hit.0),
                        ));
                    }
                }
                Phase::Dying => {
                    // Morph into the death effect. Guide sounds do not fire
                    // from here — `audio::schedule_guide_sounds` fires them
                    // from the chart clock.
                    if let Some(children) = children {
                        for child in children.iter() {
                            commands.entity(child).despawn();
                        }
                    }
                    if matches!(kind, NoteKind::Touch { .. }) {
                        // Hide the centre sprite; the burst plays out as child entities.
                        if let Some(sprite) = sprite.as_deref_mut() {
                            set_alpha(sprite, 0.0);
                        }
                        commands
                            .entity(entity)
                            .with_children(|p| spawn_touch_spark(p, &assets));
                    } else if let Some(sprite) = sprite.as_deref_mut() {
                        // Tap / SlideStar carry their own sprite: swap it in place for
                        // the hexagon pop effect.
                        *sprite = hexagon_sprite(&assets);
                    } else if matches!(kind, NoteKind::TouchHold { .. } | NoteKind::TapHold { .. })
                    {
                        // Hold parents have no sprite of their own — attach the pop.
                        commands.entity(entity).insert(hexagon_sprite(&assets));
                    }
                }
                _ => {}
            }
        }
        *timing = NoteTiming::at(now);

        // Each phase's body sets everything its phase shows, not only what
        // changes during it: a seek places notes straight into a later phase,
        // and the bodies of the phases before it never ran.
        match &*timing {
            NoteTiming::Growing(timer) => {
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
            }
            NoteTiming::Moving(timer) => {
                let t = timer.fraction();
                if !is_slide(kind) {
                    transform.scale = Vec3::ONE;
                } else {
                    grow_slide(
                        1.0,
                        children,
                        &mut slide_elements,
                        &mut slide_arrows,
                        &mut note_halos,
                    );
                }
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
                    bpm,
                    note_speed,
                    children,
                    &mut transform,
                    &mut hold_elements,
                    &layout,
                );
            }
            NoteTiming::Holding(timer) => {
                let t = timer.fraction();
                transform.scale = Vec3::ONE;
                move_tap(&mut transform, kind, 1.0, &layout);
                move_triangles(kind, 1.0, children, &mut triangles);
                match kind {
                    NoteKind::TapHold { .. } => {
                        hold_tap(
                            kind,
                            t,
                            timer,
                            bpm,
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
            }
            NoteTiming::Waiting(timer) => {
                grow_slide(
                    1.0,
                    children,
                    &mut slide_elements,
                    &mut slide_arrows,
                    &mut note_halos,
                );
                hide_slide_head(children, &mut slide_elements);
                wait_slide(timer.fraction(), children, &mut slide_elements);
            }
            NoteTiming::Sliding(timer) => {
                grow_slide(
                    1.0,
                    children,
                    &mut slide_elements,
                    &mut slide_arrows,
                    &mut note_halos,
                );
                hide_slide_head(children, &mut slide_elements);
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
            }
            NoteTiming::Dying(timer) => {
                let t = timer.fraction();
                if matches!(kind, NoteKind::Touch { .. }) {
                    // Burst children are spawned via command, so `children` only
                    // includes them from the second frame onward.
                    animate_touch_spark(t, children, &mut touch_sparks);
                } else {
                    move_tap(&mut transform, kind, 1.0, &layout);
                    // Pop: scale 1.0 -> 2.0 -> 1.0, alpha 0% -> 100% -> 0%.
                    let wave = (t * std::f32::consts::PI).sin();
                    transform.scale = Vec3::splat(1.0 + wave);
                    if let Some(sprite) = sprite.as_deref_mut() {
                        set_alpha(sprite, wave);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::visual::component::HitTime;

    fn first_kind(chart: &str) -> NoteKind {
        simai::parse_chart(chart)
            .unwrap()
            .into_iter()
            .find_map(|event| match event {
                simai::ChartEvent::NoteGroup(notes) => Some(notes[0].kind.clone()),
                _ => None,
            })
            .expect("a note in the chart")
    }

    /// A paused chart at `elapsed`, with `update_movement` and nothing else.
    fn app_at(elapsed: f64) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<NoteAssets>()
            .init_resource::<ButtonLayout>()
            .init_resource::<ChartPlayback>()
            .add_systems(Update, update_movement);
        let mut chart = app.world_mut().resource_mut::<ChartPlayback>();
        chart.pause();
        chart.sync_to_audio_position(elapsed);
        app
    }

    /// A note as `spawn_note` leaves it: fresh, in its first phase.
    fn spawn(app: &mut App, kind: NoteKind, hit: f64) -> Entity {
        app.world_mut()
            .spawn((
                Sprite::default(),
                Transform::default(),
                NoteTiming::Growing(Timer::from_seconds(0.1, TimerMode::Once)),
                kind,
                Visibility::default(),
                HitTime(hit),
                NoteBpm(120.0),
            ))
            .id()
    }

    fn count<C: Component>(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<C>>()
            .iter(app.world())
            .count()
    }

    /// A hold caught by pause on the frame it starts holding: its first-frame
    /// setup (the halo) must run once, not on every paused frame (final
    /// review, visualizer-majdata piece 1).
    #[test]
    fn a_paused_hold_spawns_its_halo_once() {
        let mut app = app_at(0.0);
        spawn(&mut app, first_kind("(120){4}1h[4:1],E"), 0.0);
        for _ in 0..5 {
            app.update();
        }
        let halos = count::<HoldHalo>(&mut app);
        assert!(halos <= 1, "{halos} halos spawned while paused");
    }

    #[test]
    fn a_note_sits_where_the_clock_puts_it() {
        // Note speed 7: Moving lasts 2/7 s and ends at the hit, so half-way
        // through it is 1/7 s before the hit.
        let mut app = app_at(2.0 - 1.0 / 7.0);
        let tap = spawn(&mut app, first_kind("(120){4}1,E"), 2.0);
        app.update();
        match app.world().get::<NoteTiming>(tap).unwrap() {
            NoteTiming::Moving(timer) => assert!((timer.fraction() - 0.5).abs() < 1e-3),
            _ => panic!("not moving"),
        }
    }

    #[test]
    fn a_paused_note_stays_put() {
        let mut app = app_at(2.0 - 1.0 / 7.0);
        let tap = spawn(&mut app, first_kind("(120){4}1,E"), 2.0);
        app.update();
        let first = *app.world().get::<Transform>(tap).unwrap();
        for _ in 0..3 {
            app.update();
            assert_eq!(*app.world().get::<Transform>(tap).unwrap(), first);
        }
    }

    #[test]
    fn a_note_past_its_end_is_gone() {
        let mut app = app_at(3.0);
        let tap = spawn(&mut app, first_kind("(120){4}1,E"), 2.0);
        app.update();
        assert!(app.world().get_entity(tap).is_err());
    }

    /// Review Focus 1 (piece 2): a hold reached mid-way, as a seek leaves it.
    #[test]
    fn a_hold_entered_mid_way_shows_one_halo() {
        let mut app = app_at(1.0);
        spawn(&mut app, first_kind("(120){4}1h[1:1],E"), 0.0);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(count::<HoldHalo>(&mut app), 1);
    }

    // ── A seek places notes straight into a later phase (final review,
    // piece 2): each phase must stand on its own, not rely on earlier ones.

    /// The chart paused at `t` after a seek, through the real spawn path.
    fn seeked(chart: &str, t: f64) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<NoteAssets>()
            .init_resource::<ButtonLayout>()
            .init_resource::<ChartPlayback>()
            .add_systems(
                Update,
                (crate::systems::visual::next_event, update_movement).chain(),
            );
        {
            let mut playback = app.world_mut().resource_mut::<ChartPlayback>();
            playback.load_chart(simai::parse_chart(chart).unwrap());
            playback.pause();
            playback.seek(t);
        }
        app.update();
        app.update();
        app
    }

    fn ring_distance(tf: &Transform) -> f32 {
        tf.translation.truncate().length() / RADIUS
    }

    #[test]
    fn a_hold_seeked_mid_hold_shows_on_the_ring() {
        let mut app = seeked("(120){4}1h[1:1],E", 1.0);
        let mut holds = app.world_mut().query::<(&Transform, &NoteKind)>();
        let (tf, _) = holds
            .iter(app.world())
            .find(|(_, k)| matches!(k, NoteKind::TapHold { .. }))
            .expect("the hold");
        assert_eq!(tf.scale, Vec3::ONE);
        assert!(
            (ring_distance(tf) - 1.0).abs() < 0.05,
            "{}",
            ring_distance(tf)
        );
    }

    #[test]
    fn a_tap_seeked_mid_pop_pops_on_the_ring() {
        let mut app = seeked("(120){4}1,E", 0.1);
        let mut taps = app.world_mut().query::<(&Transform, &NoteTiming)>();
        let (tf, timing) = taps.iter(app.world()).next().expect("the tap");
        assert_eq!(timing.phase(), Phase::Dying);
        assert!(
            (ring_distance(tf) - 1.0).abs() < 0.05,
            "{}",
            ring_distance(tf)
        );
    }

    #[test]
    fn a_slide_seeked_mid_trace_shows_its_star() {
        // Hit at 0, a beat's wait (0.5 s), then a 2 s trace.
        let mut app = seeked("(120){4}1-5[2:1],E", 1.0);
        let mut stars = app.world_mut().query::<(&SlideElement, &Visibility)>();
        let star = stars
            .iter(app.world())
            .find(|(el, _)| matches!(el, SlideElement::TraceStar(_)))
            .expect("the trace star");
        assert_eq!(*star.1, Visibility::Visible);
    }

    #[test]
    fn a_slide_seeked_mid_approach_shows_its_head_and_arrows() {
        // Hit at 2.0; at note speed 7 it moves for the last 2/7 s.
        let mut app = seeked("(120){4},,,,1-5[2:1],E", 1.9);
        let mut heads = app.world_mut().query::<(&SlideElement, &Transform)>();
        let (_, head) = heads
            .iter(app.world())
            .find(|(el, _)| matches!(el, SlideElement::Head))
            .expect("the head");
        assert_eq!(head.scale, Vec3::ONE);
        let mut arrows = app.world_mut().query::<(&SlideArrow, &Sprite)>();
        assert!(arrows.iter(app.world()).count() > 0);
        for (_, sprite) in arrows.iter(app.world()) {
            assert_eq!(sprite.color.alpha(), 1.0);
        }
    }

    #[test]
    fn a_touch_hold_seeked_mid_hold_has_its_triangles_closed() {
        let mut app = seeked("(120){4}Ch[1:1],E", 1.0);
        let mut triangles = app.world_mut().query::<(&TouchElement, &Transform)>();
        let mut seen = 0;
        for (el, tf) in triangles.iter(app.world()) {
            if matches!(el, TouchElement::Triangle) {
                seen += 1;
                assert!(tf.translation.truncate().length() < 1e-3);
            }
        }
        assert!(seen > 0);
    }

    /// The slide head's pop is spawned by a command and drawn once before
    /// update_movement first places it: it must start invisible, not flash
    /// at full size (final review M6).
    #[test]
    fn the_slide_head_pop_does_not_flash() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<NoteAssets>()
            .init_resource::<ButtonLayout>()
            .init_resource::<ChartPlayback>()
            .add_systems(
                Update,
                (crate::systems::visual::next_event, update_movement).chain(),
            );
        {
            let mut playback = app.world_mut().resource_mut::<ChartPlayback>();
            playback.load_chart(simai::parse_chart("(120){4}1-5[2:1],E").unwrap());
            playback.pause();
            playback.seek(0.05);
        }
        app.update(); // spawns the slide, which enters Waiting and queues its pop
        let mut pops = app.world_mut().query::<(&NoteKind, &Sprite, &HitTime)>();
        let (_, sprite, _) = pops
            .iter(app.world())
            .find(|(k, _, _)| matches!(k, NoteKind::Tap(_)))
            .expect("the pop");
        assert_eq!(sprite.color.alpha(), 0.0);
    }
}
