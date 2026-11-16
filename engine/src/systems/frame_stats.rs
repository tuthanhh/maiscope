//! Frame-pacing diagnostics, published once a second for JS to read.
//!
//! Exists for one reason: the visualizer is jerky on phones and nobody can see
//! a frame time on a phone. `apps/host` polls `wasm_bridge::frame_stats()` and
//! draws the numbers over the canvas when the page is opened with `?debug`.
//! See `docs/work/mobile-webview-spike/issues/02-mobile-visualizer-known-problems.md`.
//!
//! Measured against `Time<Real>`, not the virtual clock the chart runs on:
//! virtual time clamps a frame at 250ms (`Time<Virtual>::DEFAULT_MAX_DELTA`),
//! which is exactly the stall this is meant to expose.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// A frame slower than this is visibly late at 30 FPS — the floor below which
/// a phone reads as "jerky".
const SLOW_FRAME_MS: f64 = 33.4;
/// A frame slower than this starves the web-audio buffer and is heard as a
/// click or a dropped guide sound.
const STALL_FRAME_MS: f64 = 100.0;
/// How much wall time one snapshot covers.
const WINDOW_MS: f64 = 1000.0;

/// Accumulates frame deltas over a window and emits one [`Snapshot`] per
/// window. Pure — no Bevy types — so the rolling arithmetic is testable.
#[derive(Debug, Default, Resource)]
pub struct FrameStats {
    frames: u32,
    total_ms: f64,
    max_ms: f64,
    slow_frames: u32,
    stalled_frames: u32,
}

/// One second of frame pacing, as numbers a phone can show.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub fps: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub frames: u32,
    /// Frames over [`SLOW_FRAME_MS`].
    pub slow_frames: u32,
    /// Frames over [`STALL_FRAME_MS`].
    pub stalled_frames: u32,
}

impl FrameStats {
    /// Record one frame. Returns a snapshot once the window has filled.
    pub fn record(&mut self, dt_ms: f64) -> Option<Snapshot> {
        self.frames += 1;
        self.total_ms += dt_ms;
        self.max_ms = self.max_ms.max(dt_ms);
        if dt_ms > SLOW_FRAME_MS {
            self.slow_frames += 1;
        }
        if dt_ms > STALL_FRAME_MS {
            self.stalled_frames += 1;
        }

        if self.total_ms < WINDOW_MS {
            return None;
        }

        let snapshot = Snapshot {
            fps: self.frames as f64 * 1000.0 / self.total_ms,
            avg_ms: self.total_ms / self.frames as f64,
            max_ms: self.max_ms,
            frames: self.frames,
            slow_frames: self.slow_frames,
            stalled_frames: self.stalled_frames,
        };
        *self = Self::default();
        Some(snapshot)
    }
}

impl Snapshot {
    /// Hand-rolled JSON: the engine has no serde, and five numbers do not
    /// justify adding it.
    /// `msaa` is the sample count on the main camera (1 = off), so the
    /// `?msaa=` toggle can be seen to have applied.
    pub fn to_json(
        &self,
        canvas: (u32, u32),
        scale_factor: f32,
        msaa: u32,
        audio_lead_ms: f32,
    ) -> String {
        format!(
            r#"{{"fps":{:.1},"avg_ms":{:.1},"max_ms":{:.1},"frames":{},"slow_frames":{},"stalled_frames":{},"canvas":"{}x{}","scale_factor":{:.2},"msaa":{},"audio_lead_ms":{:.0}}}"#,
            self.fps,
            self.avg_ms,
            self.max_ms,
            self.frames,
            self.slow_frames,
            self.stalled_frames,
            canvas.0,
            canvas.1,
            scale_factor,
            msaa,
            audio_lead_ms,
        )
    }
}

/// Runs every frame in every state: the booting phase and the loading phase
/// are frames too, and a stall there is still a stall.
pub fn sample_frame(
    time: Res<Time<Real>>,
    mut stats: ResMut<FrameStats>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<Option<&Msaa>, With<crate::plugins::camera::MainCamera>>,
    audio_lead: Res<crate::systems::audio::AudioLead>,
) {
    let Some(snapshot) = stats.record(time.delta_secs_f64() * 1000.0) else {
        return;
    };
    let (canvas, scale) = window
        .single()
        .map(|w| ((w.physical_width(), w.physical_height()), w.scale_factor()))
        .unwrap_or(((0, 0), 0.0));
    // A camera without an explicit Msaa component renders at Bevy's default.
    let msaa = camera
        .single()
        .ok()
        .flatten()
        .copied()
        .unwrap_or_default()
        .samples();
    crate::wasm_bridge::publish_frame_stats(snapshot.to_json(
        canvas,
        scale,
        msaa,
        audio_lead.wall_ms,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing is emitted until a full second of frames has been seen, and
    /// the window then resets rather than sliding — each snapshot describes
    /// disjoint frames.
    #[test]
    fn emits_once_per_window_and_resets() {
        let mut stats = FrameStats::default();
        for _ in 0..59 {
            assert_eq!(stats.record(16.7), None);
        }
        let first = stats
            .record(16.7)
            .expect("60 frames at 16.7ms fill a second");
        assert_eq!(first.frames, 60);
        assert!((first.fps - 59.9).abs() < 0.1, "{first:?}");
        assert_eq!(first.slow_frames, 0);
        assert_eq!(first.stalled_frames, 0);

        // The next window starts from zero: one frame does not emit, and the
        // previous max does not leak forward.
        assert_eq!(stats.record(5.0), None);
        assert_eq!(stats.max_ms, 5.0);
    }

    /// A single stalled frame is counted in both buckets and dominates `max`,
    /// which is the signature of the audio starvation this exists to show.
    #[test]
    fn a_stall_is_counted_as_slow_and_stalled() {
        let mut stats = FrameStats::default();
        for _ in 0..10 {
            stats.record(16.0);
        }
        // 160ms so far; one 900ms stall crosses the window.
        let snap = stats.record(900.0).expect("window filled");
        assert_eq!(snap.frames, 11);
        assert_eq!(snap.slow_frames, 1);
        assert_eq!(snap.stalled_frames, 1);
        assert_eq!(snap.max_ms, 900.0);
        assert!(snap.fps < 11.0, "{snap:?}");
    }

    /// A steady 25 FPS phone: every frame slow, none stalled. The two buckets
    /// tell "uniformly slow" apart from "fine with hitches".
    #[test]
    fn uniformly_slow_frames_are_slow_but_not_stalled() {
        let mut stats = FrameStats::default();
        let mut snap = None;
        while snap.is_none() {
            snap = stats.record(40.0);
        }
        let snap = snap.unwrap();
        assert_eq!(snap.slow_frames, snap.frames);
        assert_eq!(snap.stalled_frames, 0);
        assert!((snap.fps - 25.0).abs() < 0.01, "{snap:?}");
    }

    #[test]
    fn json_is_flat_and_has_every_field() {
        let snap = Snapshot {
            fps: 59.94,
            avg_ms: 16.68,
            max_ms: 21.0,
            frames: 60,
            slow_frames: 0,
            stalled_frames: 0,
        };
        assert_eq!(
            snap.to_json((1170, 2532), 3.0, 4, 120.0),
            r#"{"fps":59.9,"avg_ms":16.7,"max_ms":21.0,"frames":60,"slow_frames":0,"stalled_frames":0,"canvas":"1170x2532","scale_factor":3.00,"msaa":4,"audio_lead_ms":120}"#
        );
    }
}
