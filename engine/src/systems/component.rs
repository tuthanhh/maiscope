use bevy::prelude::Component;

// The components used by the whole viewer system.
pub type ButtonId = usize;
pub type Divider = usize;
pub type Count = usize;
pub type TouchValue = usize;
pub type TouchArea = char;

#[derive(Debug, Clone)]
pub struct TimedEvent {
    pub time: f64,
    pub event: ChartEvent,
    pub bpm: f32,
}

#[derive(Debug, Clone)]
pub enum ChartEvent {
    BpmChange(f32),
    ResolutionChange(u32),
    AbsoluteLength(f64),
    NoteGroup(Vec<Note>),
    Rest,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Note {
    /// Applies to Taps, Holds, Touches, and Slide Stars (Heads).
    /// Ignored if the note is a `HeadlessSlide`.
    pub is_break: bool,
    pub is_firework: bool,
    pub is_ex: bool,
    pub kind: NoteKind,
}

#[derive(Debug, Clone)]
pub struct SlideSegment {
    pub shape: SlideShape,
    pub duration: Duration,
    /// The break modifier for the specific tracing path (independent of the star).
    pub is_break: bool,
}

#[derive(Debug, Clone, Component)]
pub enum NoteKind {
    Tap(ButtonId),
    TapHold {
        button: ButtonId,
        duration: Duration,
    },
    Touch {
        value: TouchValue,
        group: TouchArea,
    },
    TouchHold {
        value: TouchValue,
        group: TouchArea,
        duration: Duration,
    },

    /// A star head with NO path. Acts like a Tap, but is visually a star.
    SlideStar(ButtonId),

    /// A slide path with NO star head. (e.g., a path you just trace without an initial tap).
    HeadlessSlide {
        /// The anchor point where the path begins.
        start_button: ButtonId,
        segments: Vec<SlideSegment>,
        shared_duration: bool,
    },

    /// A complete standard slide containing both a star head and tracing paths.
    Slide {
        head_button: ButtonId,
        segments: Vec<SlideSegment>,
        shared_duration: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Duration {
    // The above duration can be used for all type of "holding" note elements.
    Simple {
        divider: Divider,
        count: Count,
    },
    BpmOverride {
        bpm: f32,
        divider: Divider,
        count: Count,
    },
    BpmOverrideSeconds {
        bpm: f32,
        seconds: f32,
    },
    // Specialy designed for slide.
    ExplicitWaitAndTrace {
        wait_seconds: f32,
        trace_seconds: f32,
    },
    ExplicitWaitBeats {
        wait_seconds: f32,
        divider: Divider,
        count: Count,
    },
    ExplicitWaitBpmBeats {
        wait_seconds: f32,
        bpm: f32,
        divider: Divider,
        count: Count,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum SlideShape {
    Straight {
        end: ButtonId,
    },
    ShortArc {
        end: ButtonId,
    },
    ClockwiseArc {
        end: ButtonId,
    },
    CounterClockwiseArc {
        end: ButtonId,
    },
    VShape {
        end: ButtonId,
    },
    PShape {
        end: ButtonId,
    },
    QShape {
        end: ButtonId,
    },
    GrandVShape {
        mid: ButtonId,
        end: ButtonId,
    },
    GrandPShape {
        end: ButtonId,
    },
    GrandQShape {
        end: ButtonId,
    },
    Thunderbolt {
        end: ButtonId,
        is_z: bool,
    },
    FanShape {
        ends: (ButtonId, ButtonId, ButtonId),
    },
}
