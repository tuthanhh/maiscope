//! Slide path geometry and tracing. Split into:
//! - [`geometry`]: pure button/tangent/polyline math
//! - [`generators`]: per-shape waypoint generation
//! - [`trace`]: timing + trace assembly sampled by the movement system
//!
//! Not yet fully wired into spawning.
#![allow(dead_code)]

mod generators;
mod geometry;
mod trace;

// Public API in active use by spawning / movement.
pub use geometry::get_transform_at_distance;
pub use trace::{build_slide_trace, trace_distance, trace_total_secs};

// Part of the module's public surface, but not yet wired into spawning.
#[allow(unused_imports)]
pub use generators::generate_points;
#[allow(unused_imports)]
pub use geometry::{
    calculate_total_length, generate_arc_points, generate_multi_segment_points,
    generate_offset_arc_points,
};
#[allow(unused_imports)]
pub use trace::trace_position;
