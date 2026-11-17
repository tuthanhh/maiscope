//! simai chart parser: chart text in, a list of [`ChartEvent`]s out.
//!
//! No Bevy, no I/O; `regex` is the only dependency. `engine` turns on the
//! `bevy` feature to use [`NoteKind`] as an ECS component.
//!
//! Split by responsibility:
//! - [`chart`]    — file → tokens → events, meta-token stripping.
//! - [`note`]     — dispatch + tap / touch / hold notes.
//! - [`slide`]    — slide patterns, shapes, segments, star chains.
//! - [`duration`] — `[...]` duration-bracket parsing.
//!
//! Tests live inline in each module, because the interesting entry points
//! (`parse_note`, `parse_slide_note`, `parse_duration_bracket`) are private to
//! this module tree — only `parse_chart` is public. `tests/` therefore
//! covers the corpus end-to-end and nothing else.

mod chart;
mod duration;
mod error;
mod note;
mod slide;
#[cfg(test)]
mod testutil;
mod types;

pub use chart::parse_chart;
pub use error::ParseError;
pub use types::{
    ButtonId, ChartEvent, Count, Divider, Duration, Note, NoteKind, SlideSegment, SlideShape,
    TouchArea, TouchValue,
};
