// systems/audio.rs
use crate::systems::component::{ChartEvent, NoteKind, TimedEvent};
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

/// One guide sound to play: every audible note that shares a hit time is one
/// message with `hits` counting them, so a chord is one louder sound and two
/// notes 16ms apart are two sounds — even when both fall in the same frame.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct PlayGuideSoundMessage {
    /// Chart time of the hit, seconds.
    pub time: f64,
    /// Audible notes at that time.
    pub hits: usize,
}

#[derive(Resource)]
pub struct Bgm;

#[derive(Resource)]
pub struct Sfx;

// Guide-tap SFX baked into the binary so it works in a browser without serving
// an assets dir. It's ALWAYS loaded (independent of the optional
// song BGM) so hit sounds play even for a chart loaded without audio.
const GUIDE_TAP_SFX: &[u8] = include_bytes!("../../assets/system_sounds/answer.wav");
/// Metronome click, baked in like the guide SFX.
const METRONOME_SFX: &[u8] = include_bytes!("../../assets/system_sounds/clock.wav");

// Volumes are **decibels**, not amplitudes. bevy_kira_audio 0.25's
// `with_volume` takes kira's `Decibels`, and a bare `f32` converts 1:1 — so the
// old `0.4` "duck" was +0.4 dB and the old `1.5` guide gain was +1.5 dB. Both
// SFX files peak at exactly 0 dBFS, and kira's renderer hard-clamps the mix at
// ±1.0, so any positive value here clipped every single hit. That is what the
// guide taps sounded like through a phone speaker (mobile-webview-spike 02,
// P2 fault b). 0 dB is unity; everything below stays negative on purpose.
/// BGM sits well under the hit SFX so the guide taps cut through.
const BGM_DB: f32 = -8.0;
/// Base guide-SFX level, how much each extra simultaneous hit adds, and the
/// ceiling. The ceiling is negative: a chord must not clip either.
const GUIDE_SFX_DB: f32 = -6.0;
const GUIDE_SFX_PER_HIT_DB: f32 = 0.5;
const GUIDE_SFX_MAX_DB: f32 = -3.0;
/// Metronome click level.
const METRONOME_DB: f32 = -8.0;

// Compile-time: no fixed level may ever be at or above unity again.
const _: () = assert!(BGM_DB < 0.0 && METRONOME_DB < 0.0 && GUIDE_SFX_MAX_DB < 0.0);

/// Guide-SFX level for `hit_count` simultaneous hits: louder for chords, never
/// above [`GUIDE_SFX_MAX_DB`].
fn guide_sfx_db(hit_count: usize) -> f32 {
    let extra = hit_count.saturating_sub(1) as f32 * GUIDE_SFX_PER_HIT_DB;
    (GUIDE_SFX_DB + extra).min(GUIDE_SFX_MAX_DB)
}

/// How far ahead of the visual hit a guide sound is *requested*, in wall
/// milliseconds. Zero on desktop. On the web, cpal's `webaudio` backend
/// double-buffers 2048 frames (43ms each at 48kHz) and refills from the main
/// thread, and Android adds its own output latency on top, so a sound asked
/// for at the hit lands audibly late (P2 fault a). Set from JS
/// (`set_audio_lead_ms`) while the right value is being measured.
#[derive(Resource, Default)]
pub struct AudioLead {
    pub wall_ms: f32,
}

impl AudioLead {
    /// The lead in chart-time seconds — the units of `elapsed_time` and
    /// `TimedEvent::time`. Wall time scales by `chart_speed` to become chart
    /// time, so a 100ms lead at half speed is 50ms of chart.
    pub fn chart_secs(&self, chart_speed: f32) -> f64 {
        self.wall_ms as f64 / 1000.0 * chart_speed as f64
    }
}

/// Cursor over the chart's timed events that fires guide sounds from the
/// **clock**, not from the note sprites' timers. One place decides when a hit
/// is audible, it can run ahead of the visuals by [`AudioLead`], and it does
/// not depend on an entity reaching the end of a `Timer` on the right frame.
#[derive(Resource, Default)]
pub struct GuideScheduler {
    next: usize,
    last_horizon: f64,
    /// Release times (chart seconds) of holds that have started but not yet
    /// ended. Not in the event list, because a hold's end is not an event:
    /// it is its start plus its duration, and it can fall after any number
    /// of later groups.
    releases: Vec<f64>,
}

/// Two sounds closer than this are one hit time.
const SAME_TIME_SECS: f64 = 1e-3;

impl GuideScheduler {
    /// Advance the cursor to `horizon` (chart seconds, already including any
    /// lead) and return the sounds that became due: one per hit time that
    /// has at least one audible note, carrying the count. A horizon that
    /// moved backwards is a rewind: the cursor restarts from the top so the
    /// same notes sound again on a restart.
    ///
    /// One entry per *time*, not one per frame: at 60 FPS two notes 16ms
    /// apart can become due in the same frame, and collapsing them into one
    /// sound is what "missing hits" sounds like in a dense stream
    /// (mobile-webview-spike 02, P2 fault c).
    ///
    /// A hold sounds twice, at its start and at its release — the behaviour
    /// of the sprite-timer path this replaced, which fired on entering
    /// `Holding` and again on entering `Dying`. A release due at the same
    /// time as another hit joins it as one louder sound.
    pub fn advance(&mut self, events: &[TimedEvent], horizon: f64) -> Vec<PlayGuideSoundMessage> {
        if horizon < self.last_horizon - SAME_TIME_SECS {
            self.next = 0;
            self.releases.clear();
        }
        self.last_horizon = horizon;

        let mut due = Vec::new();
        while let Some(event) = events.get(self.next) {
            if event.time > horizon {
                break;
            }
            if let ChartEvent::NoteGroup(notes) = &event.event {
                let hits = notes.iter().filter(|n| is_audible(&n.kind)).count();
                if hits > 0 {
                    due.push(PlayGuideSoundMessage {
                        time: event.time,
                        hits,
                    });
                }
                for note in notes {
                    if let NoteKind::TapHold { duration, .. }
                    | NoteKind::TouchHold { duration, .. } = &note.kind
                    {
                        self.releases
                            .push(event.time + duration.hold_secs(event.bpm) as f64);
                    }
                }
            }
            self.next += 1;
        }

        // Releases are pushed in start order, which is not release order, so
        // take every one that has come due rather than stopping at the first.
        let mut i = 0;
        while i < self.releases.len() {
            if self.releases[i] <= horizon {
                due.push(PlayGuideSoundMessage {
                    time: self.releases.swap_remove(i),
                    hits: 1,
                });
            } else {
                i += 1;
            }
        }

        coalesce(due)
    }

    /// Jump the cursor to `landed` (the chart time a seek landed on, no lead)
    /// without sounding what lies before it: the next `advance` sounds the
    /// hits from there on — the note the seek landed on, and any its lead
    /// already covers — and the releases of holds still held.
    pub fn seek(&mut self, events: &[TimedEvent], landed: f64) {
        // A seek to a measure start lands on its downbeat give or take the
        // timeline's rounding; the note there still sounds.
        let from = landed - SAME_TIME_SECS;
        self.next = events.partition_point(|e| e.time < from);
        self.releases = events[..self.next]
            .iter()
            .flat_map(|event| match &event.event {
                ChartEvent::NoteGroup(notes) => notes
                    .iter()
                    .filter_map(|note| match &note.kind {
                        NoteKind::TapHold { duration, .. }
                        | NoteKind::TouchHold { duration, .. } => {
                            Some(event.time + duration.hold_secs(event.bpm) as f64)
                        }
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .filter(|release| *release >= from)
            .collect();
        self.last_horizon = from;
    }

    /// Forget the chart: the next `advance` starts from the first event.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Sort by time and merge entries closer than `SAME_TIME_SECS` into one
/// sound carrying the summed hit count.
fn coalesce(mut due: Vec<PlayGuideSoundMessage>) -> Vec<PlayGuideSoundMessage> {
    due.sort_by(|a, b| a.time.total_cmp(&b.time));
    let mut merged: Vec<PlayGuideSoundMessage> = Vec::with_capacity(due.len());
    for sound in due {
        match merged.last_mut() {
            Some(last) if sound.time - last.time < SAME_TIME_SECS => last.hits += sound.hits,
            _ => merged.push(sound),
        }
    }
    merged
}

/// Which notes make a guide sound when they reach the ring: everything with
/// a head to tap. A slide's star head is tapped and judged like a tap, so it
/// sounds (the timer-driven path it replaced never did — an omission the
/// hexagon pop had been covering for). A `HeadlessSlide` is the second and
/// later track of a star chain, sharing the head of the first, so it stays
/// silent: one tap, one sound, however many tracks leave it.
fn is_audible(kind: &NoteKind) -> bool {
    !matches!(kind, NoteKind::HeadlessSlide { .. })
}
/// Shortest comfortable gap between metronome clicks, in **wall** seconds. The
/// beat interval is doubled (1, 2, 4, 8… beats per click) until it clears this,
/// so fast songs click once per bar and slow songs once per beat — always easy
/// to catch by ear.
const METRONOME_MIN_CLICK_SECS: f64 = 0.5;

/// The metronome's click interval (chart seconds) at `bpm` and play speed: a
/// quarter note, doubled until the *wall* gap clears the comfortable minimum.
fn click_interval_secs(bpm: f32, chart_speed: f32) -> f64 {
    let chart_speed = chart_speed.max(0.01) as f64;
    let mut interval = 60.0 / bpm.max(1.0) as f64;
    while interval / chart_speed < METRONOME_MIN_CLICK_SECS {
        interval *= 2.0;
    }
    interval
}

/// Clicks since the chart began at chart time `t`, as playback counts them:
/// each stretch at its own bpm, at the click interval in force there.
fn clicks_at(events: &[TimedEvent], t: f64, chart_speed: f32) -> f64 {
    let Some(first) = events.first() else {
        return 0.0;
    };
    let (mut clicks, mut time, mut bpm) = (0.0, 0.0, first.bpm);
    for event in events {
        if event.time >= t {
            break;
        }
        clicks += (event.time - time) / click_interval_secs(bpm, chart_speed);
        time = event.time;
        bpm = event.bpm;
    }
    clicks + (t - time) / click_interval_secs(bpm, chart_speed)
}

impl Metronome {
    /// Restart the click grid at `horizon` (chart seconds, lead included) on
    /// the chart's beat, with no click for the jump itself.
    pub fn seek(&mut self, events: &[TimedEvent], horizon: f64, chart_speed: f32) {
        self.bpm_cursor = events
            .partition_point(|e| e.time <= horizon)
            .saturating_sub(1);
        self.phase = clicks_at(events, horizon, chart_speed);
        self.last_elapsed = horizon;
    }
}

/// Always-present guide-tap SFX, decoded at startup.
#[derive(Resource)]
pub struct GuideSfx(pub Handle<AudioSource>);

/// Metronome click SFX, decoded at startup.
#[derive(Resource)]
pub struct MetronomeSfx(pub Handle<AudioSource>);

/// Metronome state. `enabled` gates the feature. The click cadence is tracked as
/// a fractional **phase** (in units of clicks) integrated from the chart clock,
/// so it stays correct across BPM changes and play-speed changes. `bpm_cursor`
/// tracks the active BPM; `last_elapsed` detects a rewind (restart/seek).
#[derive(Resource, Default)]
pub struct Metronome {
    pub enabled: bool,
    pub phase: f64,
    pub bpm_cursor: usize,
    pub last_elapsed: f64,
}

/// The optional song BGM, present only when a chart is loaded with audio.
#[derive(Resource)]
pub struct BgmSource(pub Handle<AudioSource>);

/// Handle to the currently-playing BGM instance, so transport commands
/// (restart/seek) can reach the playing audio.
#[derive(Resource)]
pub struct BgmInstance(pub Handle<AudioInstance>);

/// Decode the embedded guide SFX once at startup so it's available regardless of
/// whether a song BGM is ever loaded.
pub fn load_guide_sfx(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    match StaticSoundData::from_cursor(std::io::Cursor::new(GUIDE_TAP_SFX)) {
        Ok(sound) => {
            commands.insert_resource(GuideSfx(audio_sources.add(AudioSource { sound })));
        }
        // Non-fatal: the game still runs, just without guide hit sounds.
        Err(err) => error!("guide SFX decode failed: {err:?}"),
    }
    match StaticSoundData::from_cursor(std::io::Cursor::new(METRONOME_SFX)) {
        Ok(sound) => {
            commands.insert_resource(MetronomeSfx(audio_sources.add(AudioSource { sound })));
        }
        Err(err) => error!("metronome SFX decode failed: {err:?}"),
    }
}

/// Click once per beat, driven by the chart clock. Beat length = `60 / bpm`
/// chart-time seconds at the active BPM (so it follows BPM changes); because the
/// clock advances at `chart_speed` (and is BGM-slaved when a song plays), the
/// clicks track the play speed and stay locked to the music. Self-resets when
/// the clock rewinds (restart).
pub fn tick_metronome(
    chart: Res<crate::systems::chart_playback::ChartPlayback>,
    lead: Res<AudioLead>,
    mut metro: ResMut<Metronome>,
    sfx_channel: Res<AudioChannel<Sfx>>,
    metronome_sfx: Option<Res<MetronomeSfx>>,
) {
    if !metro.enabled || !chart.is_playing() {
        return;
    }
    let Some(sfx) = metronome_sfx else { return };
    let events = chart.timed_events();
    if events.is_empty() {
        return;
    }

    // Clicks are requested `lead` ahead so they *land* on the beat (P2).
    let elapsed = chart.elapsed_time() + lead.chart_secs(chart.chart_speed());
    let delta = elapsed - metro.last_elapsed;
    metro.last_elapsed = elapsed;

    // Rewound (restart/seek): elapsed jumped backward → restart the grid, no click.
    if delta < -1e-3 {
        metro.phase = 0.0;
        metro.bpm_cursor = 0;
        return;
    }

    // Active BPM: advance the cursor to the last event whose time has passed.
    while metro.bpm_cursor + 1 < events.len() && events[metro.bpm_cursor + 1].time <= elapsed {
        metro.bpm_cursor += 1;
    }
    let bpm = events[metro.bpm_cursor].bpm.max(1.0);

    let beat = click_interval_secs(bpm, chart.chart_speed());

    // Integrate the click *rate* (Δ chart-time / interval). A BPM/speed change
    // just changes the rate, so the phase stays tempo-correct with no reschedule.
    // One click per integer crossing (delta is one frame, so at most one).
    let prev = metro.phase;
    metro.phase += delta / beat;
    if metro.phase.floor() > prev.floor() {
        sfx_channel.play(sfx.0.clone()).with_volume(METRONOME_DB);
    }
}

/// Fire guide sounds for every hit time that has come, judged against the
/// chart clock plus [`AudioLead`]. Runs after `tick_clock` in the same chain
/// as the metronome. One message per hit time; `handle_guide_sounds` plays
/// one sound per message.
pub fn schedule_guide_sounds(
    chart: Res<crate::systems::chart_playback::ChartPlayback>,
    lead: Res<AudioLead>,
    mut scheduler: ResMut<GuideScheduler>,
    mut messages: MessageWriter<PlayGuideSoundMessage>,
) {
    if !chart.is_playing() {
        return;
    }
    let horizon = chart.elapsed_time() + lead.chart_secs(chart.chart_speed());
    for sound in scheduler.advance(chart.timed_events(), horizon) {
        messages.write(sound);
    }
}

pub fn start_bgm(
    mut commands: Commands,
    bgm_channel: Res<AudioChannel<Bgm>>,
    // Optional: a chart loaded without audio (load_chart) has no BGM and plays
    // silently against the frame clock.
    bgm_source: Option<Res<BgmSource>>,
    chart: Res<crate::systems::chart_playback::ChartPlayback>,
) {
    let Some(bgm_source) = bgm_source else {
        return;
    };

    // Lower the BGM volume slightly so the guide/hit SFX punch through. The
    // instance handle is stored so transport commands can reach the audio.
    // Honor any speed set before playback began (set_speed command) so the audio
    // rate matches the chart timing from the first frame.
    let handle = bgm_channel
        .play(bgm_source.0.clone())
        .with_volume(BGM_DB)
        .with_playback_rate(chart.chart_speed() as f64)
        .handle();
    commands.insert_resource(BgmInstance(handle));
}

pub fn handle_guide_sounds(
    mut messages: MessageReader<PlayGuideSoundMessage>,
    sfx_channel: Res<AudioChannel<Sfx>>,
    // Optional: absent if the guide SFX failed to decode at startup.
    guide_sfx: Option<Res<GuideSfx>>,
) {
    let Some(guide_sfx) = guide_sfx else {
        messages.clear();
        return;
    };

    // One sound per hit time. Louder for chords (more simultaneous hits),
    // capped below 0 dB so a full-scale sample never clips.
    for sound in messages.read() {
        sfx_channel
            .play(guide_sfx.0.clone())
            .with_volume(guide_sfx_db(sound.hits));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::component::{Duration, Note, SlideSegment};

    /// A quarter note at the fixture's 120 BPM: half a second.
    const QUARTER: Duration = Duration::Simple {
        divider: 4,
        count: 1,
    };

    fn group(time: f64, kinds: Vec<NoteKind>) -> TimedEvent {
        TimedEvent {
            time,
            event: ChartEvent::NoteGroup(
                kinds
                    .into_iter()
                    .map(|kind| Note {
                        is_break: false,
                        is_firework: false,
                        is_ex: false,
                        offset_ms: 0,
                        kind,
                    })
                    .collect(),
            ),
            bpm: 120.0,
        }
    }

    fn rest(time: f64) -> TimedEvent {
        TimedEvent {
            time,
            event: ChartEvent::Rest,
            bpm: 120.0,
        }
    }

    /// Total audible hits across the sounds an `advance` returned.
    fn hits(due: &[PlayGuideSoundMessage]) -> usize {
        due.iter().map(|s| s.hits).sum()
    }

    /// Both SFX files peak at 0 dBFS and kira hard-clamps the mix, so the
    /// only non-clipping guide level is a negative one — for one hit and for
    /// the loudest chord alike. This is the P2 fault (b) regression test.
    #[test]
    fn guide_level_never_reaches_unity() {
        assert_eq!(guide_sfx_db(1), GUIDE_SFX_DB);
        assert!(guide_sfx_db(2) > guide_sfx_db(1), "a chord is louder");
        for hits in 1..=16 {
            assert!(
                guide_sfx_db(hits) < 0.0,
                "{hits} hits: {}",
                guide_sfx_db(hits)
            );
            assert!(guide_sfx_db(hits) <= GUIDE_SFX_MAX_DB);
        }
    }

    /// A hit sounds exactly once, on the frame its time is reached, and not
    /// again on later frames.
    #[test]
    fn each_hit_fires_once() {
        let events = vec![group(0.5, vec![NoteKind::Tap(1)]), rest(1.0)];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.4)), 0);
        assert_eq!(
            s.advance(&events, 0.5),
            vec![PlayGuideSoundMessage { time: 0.5, hits: 1 }]
        );
        assert_eq!(hits(&s.advance(&events, 0.6)), 0);
        assert_eq!(hits(&s.advance(&events, 5.0)), 0, "rests are silent");
    }

    /// Two hits 16ms apart that both become due in one frame are two sounds,
    /// not one louder sound. This is the P2 fault (c) regression test: the
    /// old per-frame count collapsed them, which in a dense stream reads as
    /// hits going missing.
    #[test]
    fn distinct_hit_times_in_one_frame_are_distinct_sounds() {
        let events = vec![
            group(0.500, vec![NoteKind::Tap(1)]),
            group(0.516, vec![NoteKind::Tap(2), NoteKind::Tap(3)]),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(
            s.advance(&events, 0.52),
            vec![
                PlayGuideSoundMessage {
                    time: 0.500,
                    hits: 1
                },
                PlayGuideSoundMessage {
                    time: 0.516,
                    hits: 2
                },
            ]
        );
    }

    /// The lead is applied by the caller as a larger horizon: a 100ms lead
    /// makes the 0.5s hit due at 0.4s. The scheduler itself is lead-agnostic.
    #[test]
    fn a_larger_horizon_fires_early() {
        let events = vec![group(0.5, vec![NoteKind::Tap(1)])];
        let lead = AudioLead { wall_ms: 100.0 };
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.39 + lead.chart_secs(1.0))), 0);
        assert_eq!(hits(&s.advance(&events, 0.40 + lead.chart_secs(1.0))), 1);
        // At half speed, 100ms of wall is 50ms of chart.
        assert!((lead.chart_secs(0.5) - 0.05).abs() < 1e-9);
    }

    /// Restart rewinds the clock; the scheduler notices the horizon moving
    /// backwards and replays from the first event.
    #[test]
    fn rewind_replays_from_the_top() {
        let events = vec![
            group(0.5, vec![NoteKind::Tap(1)]),
            group(1.0, vec![NoteKind::Tap(2)]),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 1.0)), 2);
        assert_eq!(hits(&s.advance(&events, 0.0)), 0);
        assert_eq!(hits(&s.advance(&events, 0.5)), 1, "first hit sounds again");
        assert_eq!(hits(&s.advance(&events, 1.0)), 1);
    }

    /// Chords count every note with a head to tap: taps, holds, touches,
    /// star taps and slide heads. A headless track (the second and later
    /// paths of a star chain) shares the first path's head and adds nothing,
    /// so a chain with three tracks is one tap and one sound.
    #[test]
    fn chords_count_every_head_and_headless_tracks_are_silent() {
        let slide = NoteKind::Slide {
            head_button: 1,
            segments: Vec::<SlideSegment>::new(),
            shared_duration: false,
            plain_head: false,
        };
        let headless = NoteKind::HeadlessSlide {
            start_button: 1,
            segments: Vec::new(),
            shared_duration: false,
        };
        let events = vec![group(
            0.0,
            vec![
                NoteKind::Tap(1),
                NoteKind::Touch {
                    value: 1,
                    group: 'C',
                },
                NoteKind::TapHold {
                    button: 2,
                    duration: crate::systems::component::Duration::Simple {
                        divider: 4,
                        count: 1,
                    },
                },
                NoteKind::SlideStar {
                    button: 3,
                    rotates: false,
                },
                slide,
                headless,
            ],
        )];
        let mut s = GuideScheduler::default();
        assert_eq!(
            s.advance(&events, 0.0),
            vec![PlayGuideSoundMessage { time: 0.0, hits: 5 }],
            "one chord, one sound: tap, touch, hold, star tap, slide head"
        );

        // A star chain parses to one Slide plus N HeadlessSlides, all at one
        // time: exactly one hit, whatever N is.
        let chain = vec![group(
            1.0,
            vec![
                NoteKind::Slide {
                    head_button: 1,
                    segments: Vec::new(),
                    shared_duration: false,
                    plain_head: false,
                },
                NoteKind::HeadlessSlide {
                    start_button: 1,
                    segments: Vec::new(),
                    shared_duration: false,
                },
                NoteKind::HeadlessSlide {
                    start_button: 1,
                    segments: Vec::new(),
                    shared_duration: false,
                },
            ],
        )];
        let mut s = GuideScheduler::default();
        assert_eq!(
            s.advance(&chain, 1.0),
            vec![PlayGuideSoundMessage { time: 1.0, hits: 1 }]
        );
    }

    /// Loading a new chart resets the cursor even if the clock happens to
    /// read the same value as before.
    #[test]
    fn a_hold_sounds_at_its_start_and_at_its_release() {
        let events = vec![
            group(
                1.0,
                vec![NoteKind::TapHold {
                    button: 1,
                    duration: QUARTER,
                }],
            ),
            group(1.25, vec![NoteKind::Tap(2)]),
            rest(3.0),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(
            s.advance(&events, 1.0),
            vec![PlayGuideSoundMessage { time: 1.0, hits: 1 }]
        );
        assert_eq!(
            s.advance(&events, 1.3),
            vec![PlayGuideSoundMessage {
                time: 1.25,
                hits: 1
            }],
            "a tap during the hold is its own sound"
        );
        assert_eq!(
            s.advance(&events, 1.5),
            vec![PlayGuideSoundMessage { time: 1.5, hits: 1 }],
            "the release lands start + duration, after a later group"
        );
        assert_eq!(hits(&s.advance(&events, 5.0)), 0, "and only once");
    }

    #[test]
    fn a_touch_hold_releases_too_and_bpm_overrides_count() {
        let events = vec![
            group(
                0.0,
                vec![NoteKind::TouchHold {
                    value: 1,
                    group: 'C',
                    duration: Duration::BpmOverride {
                        bpm: 60.0,
                        divider: 4,
                        count: 1,
                    },
                }],
            ),
            rest(4.0),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.0)), 1);
        assert_eq!(
            hits(&s.advance(&events, 0.9)),
            0,
            "at 60 BPM a quarter is a full second"
        );
        assert_eq!(
            s.advance(&events, 1.0),
            vec![PlayGuideSoundMessage { time: 1.0, hits: 1 }]
        );
    }

    #[test]
    fn a_release_on_another_hit_is_one_louder_sound() {
        let events = vec![
            group(
                0.5,
                vec![NoteKind::TapHold {
                    button: 1,
                    duration: QUARTER,
                }],
            ),
            group(1.0, vec![NoteKind::Tap(5), NoteKind::Tap(6)]),
            rest(2.0),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.5)), 1);
        assert_eq!(
            s.advance(&events, 1.0),
            vec![PlayGuideSoundMessage { time: 1.0, hits: 3 }]
        );
    }

    #[test]
    fn rewind_drops_pending_releases() {
        let events = vec![
            group(
                0.5,
                vec![NoteKind::TapHold {
                    button: 1,
                    duration: QUARTER,
                }],
            ),
            rest(2.0),
        ];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.6)), 1);
        assert_eq!(
            hits(&s.advance(&events, 0.0)),
            0,
            "restart: nothing is due at 0"
        );
        assert_eq!(hits(&s.advance(&events, 0.6)), 1, "the start sounds again");
        assert_eq!(
            s.advance(&events, 1.0),
            vec![PlayGuideSoundMessage { time: 1.0, hits: 1 }],
            "one release, not the stale one from before the rewind as well"
        );
    }

    #[test]
    fn reset_forgets_the_cursor() {
        let events = vec![group(0.0, vec![NoteKind::Tap(1)])];
        let mut s = GuideScheduler::default();
        assert_eq!(hits(&s.advance(&events, 0.0)), 1);
        s.reset();
        assert_eq!(hits(&s.advance(&events, 0.0)), 1);
    }

    fn parsed(chart: &str) -> Vec<TimedEvent> {
        let mut playback = crate::systems::chart_playback::ChartPlayback::default();
        playback.load_chart(simai::parse_chart(chart).unwrap());
        playback.timed_events().to_vec()
    }

    /// Review Focus 3 (piece 2).
    #[test]
    fn a_seek_skips_the_guide_sounds_it_jumps_over() {
        let events = parsed("(120){4}1,1,1,1,1,1,1,1,E");
        let mut guide = GuideScheduler::default();
        guide.advance(&events, 0.1);
        guide.seek(&events, 3.0);
        let due = guide.advance(&events, 3.6);
        let times: Vec<f64> = due.iter().map(|s| s.time).collect();
        assert_eq!(times, vec![3.0, 3.5]);
    }

    /// A seek to a measure start lands on its downbeat: the note there, and
    /// any within the guide lead after it, still sound (final review M1).
    #[test]
    fn a_seek_sounds_the_note_it_lands_on() {
        let events = parsed("(120){4}1,1,1,1,1,1,1,1,E");
        let mut guide = GuideScheduler::default();
        // The timeline's measure starts are rounded: land a hair early.
        guide.seek(&events, 2.0 - 0.0004);
        let due = guide.advance(&events, 2.0 + 0.2);
        let times: Vec<f64> = due.iter().map(|s| s.time).collect();
        assert_eq!(times, vec![2.0]);
    }

    #[test]
    fn a_seek_keeps_a_hold_release_that_is_still_due() {
        let events = parsed("(120){4}1h[1:1],E");
        let mut guide = GuideScheduler::default();
        guide.seek(&events, 1.0);
        let due = guide.advance(&events, 2.1);
        assert_eq!(due.len(), 1);
        assert!((due[0].time - 2.0).abs() < 1e-9);
    }

    #[test]
    fn the_metronome_lands_on_the_beat_after_a_seek() {
        let events = parsed("(120){4}1,1,1,1,1,1,1,1,E");
        let mut metro = Metronome::default();
        metro.seek(&events, 1.25, 1.0);
        assert!((metro.phase.fract() - 0.5).abs() < 1e-9);
        assert_eq!(metro.last_elapsed, 1.25);
    }

    /// The click interval doubles at fast tempos; after a seek the grid is
    /// the one playback built, interval by interval (final review M2).
    #[test]
    fn the_metronome_counts_clicks_across_tempo_changes() {
        // 120 BPM clicks every 0.5 s for the first second; at 300 BPM a beat
        // (0.2 s) doubles to 0.8 s. At 1.4 s: 2 clicks + 0.4/0.8.
        let events = parsed("(120){4}1,1,(300)1,1,1,1,E");
        let mut metro = Metronome::default();
        metro.seek(&events, 1.4, 1.0);
        assert!((metro.phase - 2.5).abs() < 1e-9, "{}", metro.phase);
    }
}
