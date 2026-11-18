//! The slide star's path and timing (visualizer-majdata 14): built from
//! MajdataPlay's layouts ([`trace`]) and sampled with polyline helpers
//! ([`geometry`]).

mod geometry;
mod trace;

pub use geometry::get_transform_at_distance;
#[cfg(test)]
pub use trace::trace_total_secs;
pub use trace::{BuiltSlide, build_slide_from_layout, slide_span_secs, star_distance};
