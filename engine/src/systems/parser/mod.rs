//! simai chart parser.
//!
//! Split by responsibility:
//! - [`chart`]    — file → tokens → events, meta-token stripping.
//! - [`note`]     — dispatch + tap / touch / hold notes.
//! - [`slide`]    — slide patterns, shapes, segments, star chains.
//! - [`duration`] — `[...]` duration-bracket parsing.
//!
//! Tests live inline in each module, because the interesting entry points
//! (`parse_note`, `parse_slide_note`, `parse_duration_bracket`) are private to
//! this module tree — only `parse_chart` is public. `engine/tests/` therefore
//! covers the corpus end-to-end and nothing else.

mod chart;
mod duration;
mod note;
mod slide;
#[cfg(test)]
mod testutil;

pub use chart::parse_chart;
