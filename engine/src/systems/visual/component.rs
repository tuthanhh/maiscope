// ── ECS visual components ──────────────────────────────────────────────────

use bevy::prelude::*;

#[derive(Component)]
pub enum NoteTiming {
    Growing(Timer),
    Moving(Timer),
    Holding(Timer),
    Waiting(Timer),
    Sliding(Timer),
    Dying(Timer),
}

// ------------------------
// Special visual element
// ------------------------
#[derive(Component)]
pub enum HoldNoteElement {
    Head,
    Body,
    Tail,
    /// Glow dot pinned to the moving tail end (`Hold_End_dot.png`).
    EndDot,
}

#[derive(Component)]
pub enum TouchElement {
    Center,
    Triangle,
}

/// One sub-element of the touch-note death burst. Animated from the parent's
/// `Dying` timer fraction in `update_movement`.
#[derive(Component)]
pub enum TouchSpark {
    /// Expanding, fading ring.
    Halo,
    /// Star that converges to center in the first sub-phase. Payload = angle (rad).
    StarIn(f32),
    /// Star that bursts outward in the second sub-phase. Payload = angle (rad).
    StarOut(f32),
}

/// One coloured edge (`edge` 0..4, clockwise from the top corner) of the
/// touch-hold countdown diamond. Hidden until the hold begins, then revealed
/// progressively clockwise during the Holding phase.
#[derive(Component)]
pub struct TouchHoldCountdown {
    pub edge: usize,
}

#[derive(Component)]
pub struct SlidePath {
    pub waypoints: Vec<Vec2>,
    pub total_length: f32,
    /// `(cumulative_trace_time, cumulative_distance)` at each segment end.
    /// Drives the time→distance mapping for the tracing star.
    pub breakpoints: Vec<(f32, f32)>,
    /// Seconds the slide waits (Waiting phase) before the trace begins.
    pub wait_secs: f32,
}

/// A chevron arrow sitting at `distance_along_path` units along the slide track.
/// `lane` is the fan-lane index (0 for ordinary single-lane slides).
#[derive(Component)]
pub struct SlideArrow {
    pub distance_along_path: f32,
    pub lane: usize,
}

/// The diverging lanes of a fan (`w`) slide: one waypoint list + length per end.
#[derive(Component)]
pub struct FanLanes {
    pub lanes: Vec<Vec<Vec2>>,
    pub lengths: Vec<f32>,
}

/// Glowing halo child spawned around a hold head during the Holding phase.
#[derive(Component)]
pub struct HoldHalo;

/// Faint `Judge/Normal.png` ring drawn behind a tap/star note. Anchored at its
/// glow dot, which rides the note; the ring stays concentric with the judgement
/// circle, its radius growing from the spawn circle to the judgement ring as the
/// note travels outward.
#[derive(Component)]
pub struct NoteHalo;

/// Marks the two star visuals that belong to a slide note.
#[derive(Component)]
pub enum SlideElement {
    /// Initial star-tap that approaches the judgment ring, then vanishes (Slide only).
    Head,
    /// Star that traces a path during the Sliding phase. The payload is the
    /// fan-lane index (0 for ordinary single-lane slides).
    TraceStar(usize),
}

#[derive(Component)]
pub struct NoteBpm(pub f32);
