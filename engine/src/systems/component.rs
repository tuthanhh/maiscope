// The chart model lives in `simai`; re-exported here so every engine path
// (`crate::systems::component::NoteKind` and the rest) stays as it was.
// `NoteKind` is an ECS component through simai's `bevy` feature.
pub use simai::{ChartEvent, Duration, Note, NoteKind, SlideSegment, SlideShape};

/// A chart event placed on the playback clock. Playback state, not chart
/// data, so it stays in the engine.
#[derive(Debug, Clone)]
pub struct TimedEvent {
    pub time: f64,
    pub event: ChartEvent,
    pub bpm: f32,
}
