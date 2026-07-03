//! simai chart parser.
//!
//! Split by responsibility:
//! - [`chart`]    — file → tokens → events, meta-token stripping.
//! - [`note`]     — dispatch + tap / touch / hold notes.
//! - [`slide`]    — slide patterns, shapes, segments, star chains.
//! - [`duration`] — `[...]` duration-bracket parsing.

mod chart;
mod duration;
mod note;
mod slide;

pub use chart::parse_chart;
