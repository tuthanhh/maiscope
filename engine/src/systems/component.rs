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

#[derive(Debug, Clone, PartialEq)]
pub enum ChartEvent {
    BpmChange(f32),
    ResolutionChange(u32),
    AbsoluteLength(f64),
    NoteGroup(Vec<Note>),
    Rest,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Note {
    /// Applies to Taps, Holds, Touches, and Slide Stars (Heads).
    /// Ignored if the note is a `HeadlessSlide`.
    pub is_break: bool,
    pub is_firework: bool,
    pub is_ex: bool,
    /// Sub-comma delay from the pseudo-EACH backtick: `` 1`2, `` puts BUTTON-2
    /// 1ms after BUTTON-1. Zero for every ordinary note.
    ///
    /// The delay does not advance the beat grid — a token is one comma however
    /// many backticks it holds. Its visible effect is that notes at *different*
    /// offsets are not simultaneous, so they must not render as an EACH.
    pub offset_ms: u32,
    pub kind: NoteKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlideSegment {
    pub shape: SlideShape,
    pub duration: Duration,
    /// The break modifier for the specific tracing path (independent of the star).
    #[allow(dead_code)]
    pub is_break: bool,
}

#[derive(Debug, Clone, PartialEq, Component)]
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
    #[allow(dead_code)]
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
    /// An absolute length in seconds, independent of BPM: `[#5.678]`.
    /// Distinct from `BpmOverrideSeconds`, which also restates the BPM.
    Seconds(f32),
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
