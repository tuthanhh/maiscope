use super::component::{ChartEvent, TimedEvent};
use super::{DEFAULT_BPM, GROWING, MOVING};
use bevy::prelude::*;

#[derive(Resource)]
pub struct ChartPlayback {
    /// Pre-computed list of timed events with absolute timestamps.
    timed_events: Vec<TimedEvent>,
    /// Index of the next timed event to be spawned. Invariant: always stays
    /// in sync with `timed_events` — reset together via `load_chart`/`restart`,
    /// never mutated independently of them.
    next_spawn_index: usize,

    /// Speed of the note (visual travel speed multiplier).
    note_speed: f32,
    /// Playing speed (chart speed multiplier, affects both timing and visuals).
    chart_speed: f32,
    /// Is playing.
    is_playing: bool,
    /// Elapsed playback time in seconds (advances each frame).
    elapsed_time: f64,
    /// When the last note finishes (chart seconds): where a seek stops.
    length: f64,
    /// A seek's target, until `next_event` has put its notes on screen.
    respawn: Option<f64>,
    /// A seek's target, until the music reports a position near it.
    audio_target: Option<f64>,
}

/// How close the music must come to a seek's target before it drives the
/// clock again (chart seconds).
const AUDIO_CAUGHT_UP: f64 = 0.1;

impl Default for ChartPlayback {
    fn default() -> Self {
        Self {
            timed_events: Vec::new(),
            next_spawn_index: 0,
            note_speed: 7.0,
            chart_speed: 1.0,
            is_playing: true,
            elapsed_time: 0.0,
            length: 0.0,
            respawn: None,
            audio_target: None,
        }
    }
}

impl ChartPlayback {
    /// Pre-compute absolute timestamps for every event in the parsed chart.
    ///
    /// The simai timing model:
    /// - `(BPM)` sets the current BPM.
    /// - `{N}` sets the length divider (resolution).
    /// - `{#S}` sets the per-comma length directly to S seconds (absolute length mode).
    /// - Per-comma length = `240 / BPM / resolution` (seconds), unless absolute length is active.
    /// - Each comma (`Rest` or `NoteGroup`) advances time by the per-comma length.
    /// - BPM/resolution changes take effect immediately and do NOT advance time.
    fn compute_timestamps(&mut self, events: Vec<ChartEvent>) {
        let mut timed: Vec<TimedEvent> = Vec::with_capacity(events.len());
        let mut current_time: f64 = 0.0;
        let mut bpm: f32 = 240.0;
        let mut resolution: u32 = 4;
        // When `{#S}` is active, per-comma length is set directly in seconds.
        // A subsequent `{N}` (resolution change) clears this and reverts to
        // the normal `240 / BPM / resolution` formula.
        let mut absolute_comma_length: Option<f64> = None;

        for event in events {
            match &event {
                ChartEvent::BpmChange(new_bpm) => {
                    bpm = *new_bpm;
                }
                ChartEvent::ResolutionChange(new_res) => {
                    resolution = *new_res;
                    absolute_comma_length = None;
                }
                ChartEvent::AbsoluteLength(seconds) => {
                    absolute_comma_length = Some(*seconds);
                }
                ChartEvent::NoteGroup(_) | ChartEvent::Rest => {
                    timed.push(TimedEvent {
                        time: current_time,
                        event: event.clone(),
                        bpm,
                    });
                    let per_comma = absolute_comma_length
                        .unwrap_or_else(|| DEFAULT_BPM / bpm as f64 / resolution as f64);
                    current_time += per_comma;
                }
            }
        }

        self.length = crate::systems::timeline::chart_length(&timed);
        self.timed_events = timed;
        self.next_spawn_index = 0;
    }

    /// Load a freshly parsed chart: pre-computes timestamps and hard-resets
    /// playback state. Loading a new chart must flush the previous one's
    /// position, so `next_spawn_index`/`elapsed_time` reset and playback
    /// (re)starts — callers used to do this as three separate field writes
    /// (`compute_timestamps` + `elapsed_time = 0.0` + `is_playing = true`);
    /// bundling them here makes forgetting one impossible.
    pub fn load_chart(&mut self, events: Vec<ChartEvent>) {
        self.compute_timestamps(events);
        self.elapsed_time = 0.0;
        self.is_playing = true;
        self.respawn = None;
        self.audio_target = None;
    }

    /// Rewind to the start of the current chart and resume playing. Does not
    /// touch the audio instance or spawned note entities — those are a Bevy
    /// system's concern (see `apply_commands`), not this resource's.
    pub fn restart(&mut self) {
        self.next_spawn_index = 0;
        self.elapsed_time = 0.0;
        self.is_playing = true;
        self.respawn = None;
        self.audio_target = None;
    }

    /// Forget the loaded chart: no events, clock at zero, not playing. The
    /// chart and note speeds are the user's settings and survive. Notes on
    /// screen and the music are `apply_commands`' to clear (`Unload`).
    pub fn unload(&mut self) {
        self.timed_events.clear();
        self.next_spawn_index = 0;
        self.elapsed_time = 0.0;
        self.is_playing = false;
        self.length = 0.0;
        self.respawn = None;
        self.audio_target = None;
    }

    /// Jump the clock to `t` (clamped to the chart), playing or paused. Spawning
    /// resumes after the notes visible at `t`, which `next_event` puts on
    /// screen (`take_respawn`); the notes and the music are the systems'.
    pub fn seek(&mut self, t: f64) {
        let t = t.clamp(0.0, self.length);
        self.elapsed_time = t;
        self.respawn_at(t);
        self.audio_target = Some(t);
    }

    /// Put on screen what is visible at `t`, and spawn on from there.
    fn respawn_at(&mut self, t: f64) {
        let window = t + self.approach_time();
        self.next_spawn_index = self.timed_events.partition_point(|e| e.time <= window);
        self.respawn = Some(t);
    }

    /// The pending seek target, once: the notes visible there are to spawn.
    pub fn take_respawn(&mut self) -> Option<f64> {
        self.respawn.take()
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        self.is_playing = true;
    }

    /// Advance the clock by `delta_wall_secs` of real time, scaled by the
    /// current chart speed. Used when no BGM is present — the frame-driven
    /// path (see `sync_to_audio_position` for the BGM-slaved path).
    pub fn tick(&mut self, delta_wall_secs: f64) {
        self.elapsed_time += delta_wall_secs * self.chart_speed as f64;
    }

    /// Slave the clock to an external audio position (chart-time seconds).
    /// Used when a BGM is present — the BGM's playback position already
    /// equals chart-time (it plays at `chart_speed`), so this is a direct
    /// assignment, not a scaled increment.
    /// After a seek, positions far from its target are the music still
    /// reporting where it was, and are ignored until it catches up.
    pub fn sync_to_audio_position(&mut self, position: f64) {
        if let Some(target) = self.audio_target {
            if (position - target).abs() > AUDIO_CAUGHT_UP {
                return;
            }
            self.audio_target = None;
        }
        self.elapsed_time = position;
    }

    pub fn set_chart_speed(&mut self, rate: f32) {
        self.chart_speed = rate;
    }

    /// The approach length follows the note speed, so the notes on screen are
    /// placed again: a faster speed would otherwise drop notes already spawned
    /// behind the spawn cursor, where they never come back.
    pub fn set_note_speed(&mut self, rate: f32) {
        if rate != self.note_speed && self.has_events() {
            self.note_speed = rate;
            self.respawn_at(self.elapsed_time);
        }
        self.note_speed = rate;
    }

    pub fn chart_speed(&self) -> f32 {
        self.chart_speed
    }

    pub fn note_speed(&self) -> f32 {
        self.note_speed
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn elapsed_time(&self) -> f64 {
        self.elapsed_time
    }

    /// Whether a chart has been loaded (has any events at all).
    pub fn has_events(&self) -> bool {
        !self.timed_events.is_empty()
    }

    /// Whether every note has spawned: what is left, if anything, is rests,
    /// which would otherwise keep the chart running past its length.
    pub fn all_notes_spawned(&self) -> bool {
        !self.timed_events[self.next_spawn_index.min(self.timed_events.len())..]
            .iter()
            .any(|e| matches!(e.event, ChartEvent::NoteGroup(_)))
    }

    /// Read-only access to the full event list, e.g. for the metronome's BPM
    /// cursor, which needs to scan events without advancing the spawn index.
    pub fn timed_events(&self) -> &[TimedEvent] {
        &self.timed_events
    }

    /// How far in advance (in chart-time seconds) notes spawn before their hit.
    ///
    /// A note travels in `(GROWING + MOVING) / (chart_speed · note_speed)`
    /// **wall** seconds. The clock advances at `chart_speed` (chart-time per wall
    /// second), so the lead in chart-time — the units of `elapsed_time` /
    /// `event.time` — is `travel_wall · chart_speed`, and the `chart_speed`
    /// cancels: the lead is purely `(GROWING + MOVING) / note_speed`.
    pub fn approach_time(&self) -> f64 {
        (GROWING + MOVING) / self.note_speed as f64
    }

    /// Checks whether the next event is due based on the current elapsed time,
    /// and if so, advances the spawn index and returns it.
    ///
    /// This is the real-time playback path — `elapsed_time` is NOT mutated here;
    /// it is driven externally by the frame loop.
    pub fn advance(&mut self) -> Option<TimedEvent> {
        let event = self.timed_events.get(self.next_spawn_index)?;

        if self.elapsed_time + self.approach_time() >= event.time {
            self.next_spawn_index += 1;
            Some(event.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unload_empties_the_chart_and_keeps_the_user_speeds() {
        let mut chart = ChartPlayback::default();
        chart.load_chart(simai::parse_chart("(120){4}1,2,3,4,E").unwrap());
        chart.set_chart_speed(1.5);
        chart.set_note_speed(8.0);
        chart.tick(1.0);
        chart.unload();
        assert!(chart.timed_events().is_empty());
        assert_eq!(chart.elapsed_time(), 0.0);
        assert!(!chart.is_playing());
        assert_eq!(chart.chart_speed(), 1.5);
        assert_eq!(chart.note_speed(), 8.0);
    }

    fn loaded(chart: &str) -> ChartPlayback {
        let mut playback = ChartPlayback::default();
        playback.load_chart(simai::parse_chart(chart).unwrap());
        playback
    }

    #[test]
    fn seek_resumes_spawning_after_the_visible_window() {
        let mut chart = loaded("(120){4}1,1,1,1,1,1,1,1,E");
        chart.seek(2.0);
        assert_eq!(chart.elapsed_time(), 2.0);
        while let Some(event) = chart.advance() {
            assert!(event.time > 2.0 + chart.approach_time(), "{}", event.time);
        }
    }

    #[test]
    fn seek_stays_inside_the_chart() {
        let mut chart = loaded("(120){4}1,1,E");
        chart.seek(-1.0);
        assert_eq!(chart.elapsed_time(), 0.0);
        chart.seek(100.0);
        assert!((chart.elapsed_time() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn a_seek_asks_for_one_respawn() {
        let mut chart = loaded("(120){4}1,1,E");
        chart.seek(0.5);
        assert_eq!(chart.take_respawn(), Some(0.5));
        assert_eq!(chart.take_respawn(), None);
    }

    /// The music takes a moment to reach a seek: until it reports a position
    /// near the target, the clock keeps the target (or respawned notes would
    /// be placed, and despawned, at the old time).
    #[test]
    fn a_seek_ignores_the_music_until_it_catches_up() {
        let mut chart = loaded("(120){4}1,1,1,1,1,1,1,1,1,1,1,1,E");
        chart.seek(5.0);
        chart.sync_to_audio_position(1.0);
        assert_eq!(chart.elapsed_time(), 5.0);
        chart.sync_to_audio_position(5.01);
        assert_eq!(chart.elapsed_time(), 5.01);
        chart.sync_to_audio_position(5.2);
        assert_eq!(chart.elapsed_time(), 5.2);
    }

    /// Final review, piece 2: a seek still pending must not outlive a restart
    /// or a new chart — the music would be ignored until it reached the old
    /// target, and notes would respawn there.
    #[test]
    fn a_restart_or_a_new_chart_forgets_a_pending_seek() {
        let mut chart = loaded("(120){4}1,1,1,1,1,1,1,1,1,1,1,1,E");
        chart.seek(5.0);
        chart.restart();
        assert_eq!(chart.take_respawn(), None);
        chart.sync_to_audio_position(0.5);
        assert_eq!(chart.elapsed_time(), 0.5);

        chart.seek(5.0);
        chart.load_chart(simai::parse_chart("(120){4}1,1,E").unwrap());
        assert_eq!(chart.take_respawn(), None);
        chart.sync_to_audio_position(0.25);
        assert_eq!(chart.elapsed_time(), 0.25);
    }
}
