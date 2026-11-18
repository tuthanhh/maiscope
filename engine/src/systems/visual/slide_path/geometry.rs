//! Polyline sampling for the slide star's path.

use bevy::prelude::Vec2;

/// Find the exact position and facing angle at a specific distance along a polyline path.
pub fn get_transform_at_distance(points: &[Vec2], target_distance: f32) -> (Vec2, f32) {
    let mut current_dist = 0.0;

    for window in points.windows(2) {
        let p1 = window[0];
        let p2 = window[1];
        let segment_length = p1.distance(p2);

        if segment_length < f32::EPSILON {
            continue;
        }

        if current_dist + segment_length >= target_distance {
            let t = (target_distance - current_dist) / segment_length;
            let exact_pos = p1.lerp(p2, t);
            let angle = (p2.y - p1.y).atan2(p2.x - p1.x);
            return (exact_pos, angle);
        }
        current_dist += segment_length;
    }

    // Fallback: return the last point
    if points.len() >= 2 {
        let last = points[points.len() - 1];
        let prev = points[points.len() - 2];
        let angle = (last.y - prev.y).atan2(last.x - prev.x);
        (last, angle)
    } else if let Some(p) = points.last() {
        (*p, 0.0)
    } else {
        (Vec2::ZERO, 0.0)
    }
}
