//! Which MajdataPlay prefab draws a slide segment, and where its arrows go.
//! A port of MajdataPlay's `NoteLoader.DetectShapeFromText` and of the
//! placement in `SlideDrop.Init` (at the commit in
//! `engine/assets/majdata/SOURCE.md`); nothing else of its code.

use bevy::prelude::*;

use super::layouts::Bar;
use crate::systems::component::SlideShape;
use crate::systems::visual::RADIUS;

type ButtonId = usize;

/// The prefab that draws a segment, and whether it is drawn mirrored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub prefab: &'static str,
    pub mirror: bool,
}

/// `end` counted from `start`: 1 is `start` itself, 5 the key opposite.
fn relative(start: ButtonId, end: ButtonId) -> usize {
    (end + 8 - start) % 8 + 1
}

/// The relative key seen in a mirror through key 1.
fn mirror_key(key: usize) -> usize {
    match key {
        1 => 1,
        k => 10 - k,
    }
}

/// Keys 7, 8, 1 and 2.
fn is_upper_half(key: ButtonId) -> bool {
    matches!(key, 7 | 8 | 1 | 2)
}

const LINES: [&str; 9] = [
    "",
    "",
    "",
    "Star_Line_3",
    "Star_Line_4",
    "Star_Line_5",
    "Star_Line_6",
    "Star_Line_7",
    "",
];
const CIRCLES: [&str; 9] = [
    "",
    "Star_Circle_1",
    "Star_Circle_2",
    "Star_Circle_3",
    "Star_Circle_4",
    "Star_Circle_5",
    "Star_Circle_6",
    "Star_Circle_7",
    "Star_Circle_8",
];
const VS: [&str; 9] = [
    "", "Star_V_1", "Star_V_2", "Star_V_3", "Star_V_4", "", "Star_V_6", "Star_V_7", "Star_V_8",
];
const PQS: [&str; 9] = [
    "",
    "Star_pq_1",
    "Star_pq_2",
    "Star_pq_3",
    "Star_pq_4",
    "Star_pq_5",
    "Star_pq_6",
    "Star_pq_7",
    "Star_pq_8",
];
const PPQQS: [&str; 9] = [
    "",
    "Star_ppqq_1",
    "Star_ppqq_2",
    "Star_ppqq_3",
    "Star_ppqq_4",
    "Star_ppqq_5",
    "Star_ppqq_6",
    "Star_ppqq_7",
    "Star_ppqq_8",
];
const LS: [&str; 9] = [
    "", "", "Star_L_2", "Star_L_3", "Star_L_4", "Star_L_5", "", "", "",
];

fn plain(prefab: &'static str) -> Option<Placement> {
    (!prefab.is_empty()).then_some(Placement {
        prefab,
        mirror: false,
    })
}

fn mirrored(prefab: &'static str) -> Option<Placement> {
    (!prefab.is_empty()).then_some(Placement {
        prefab,
        mirror: true,
    })
}

/// The prefab for one segment starting at `start`, or `None` where
/// MajdataPlay rejects the slide (e.g. a straight one to the next key).
pub fn prefab_for(shape: &SlideShape, start: ButtonId) -> Option<Placement> {
    if !(1..=8).contains(&start) {
        return None;
    }
    let rel = |end: ButtonId| relative(start, end);
    match *shape {
        SlideShape::Straight { end } => plain(LINES[rel(end)]),
        // `>` runs clockwise from the upper half; from the lower half it is
        // the mirror image of a clockwise circle. `<` is the reverse.
        SlideShape::ClockwiseArc { end } if is_upper_half(start) => plain(CIRCLES[rel(end)]),
        SlideShape::ClockwiseArc { end } => mirrored(CIRCLES[mirror_key(rel(end))]),
        SlideShape::CounterClockwiseArc { end } if !is_upper_half(start) => {
            plain(CIRCLES[rel(end)])
        }
        SlideShape::CounterClockwiseArc { end } => mirrored(CIRCLES[mirror_key(rel(end))]),
        SlideShape::ShortArc { end } => match rel(end) {
            1 | 5 => None,
            r if r < 5 => plain(CIRCLES[r]),
            r => mirrored(CIRCLES[mirror_key(r)]),
        },
        SlideShape::VShape { end } => plain(VS[rel(end)]),
        SlideShape::PShape { end } => plain(PQS[rel(end)]),
        SlideShape::QShape { end } => mirrored(PQS[mirror_key(rel(end))]),
        SlideShape::GrandPShape { end } => plain(PPQQS[rel(end)]),
        SlideShape::GrandQShape { end } => mirrored(PPQQS[mirror_key(rel(end))]),
        SlideShape::Thunderbolt { end, .. } if rel(end) != 5 => None,
        SlideShape::Thunderbolt { is_z: false, .. } => plain("Star_S"),
        SlideShape::Thunderbolt { is_z: true, .. } => mirrored("Star_S"),
        SlideShape::GrandVShape { mid, end } => match (rel(mid), rel(end)) {
            (7, e @ 2..=5) => plain(LS[e]),
            (3, e @ 5..=8) => mirrored(LS[mirror_key(e)]),
            _ => None,
        },
        // The parser stores a fan's centre key first.
        SlideShape::FanShape {
            ends: (centre, _, _),
        } if rel(centre) == 5 => plain("Slide_Wifi"),
        SlideShape::FanShape { .. } => None,
    }
}

/// A bar's position (pixels) and rotation (radians) on our playfield, for a
/// slide starting at `start`. MajdataPlay mirrors with `localScale.x = -1`
/// under `Euler(0, 0, -45 · start)`, and otherwise turns the prefab by
/// `Euler(0, 0, -45 · (start - 1))`.
pub fn place(bar: &Bar, start: ButtonId, mirror: bool) -> (Vec2, f32) {
    let (local, angle_deg, turn_deg) = if mirror {
        (
            Vec2::new(-bar.pos.x, bar.pos.y),
            180.0 - bar.angle_deg,
            -45.0 * start as f32,
        )
    } else {
        (bar.pos, bar.angle_deg, -45.0 * (start as f32 - 1.0))
    };
    let turn = turn_deg.to_radians();
    let world = Vec2::from_angle(turn).rotate(local) * (RADIUS / 4.8);
    (world, (angle_deg + turn_deg).to_radians())
}

/// The key a segment ends on: the next segment of a chain starts there. A
/// fan ends on its centre key.
pub fn shape_end(shape: &SlideShape) -> ButtonId {
    match *shape {
        SlideShape::Straight { end }
        | SlideShape::ShortArc { end }
        | SlideShape::ClockwiseArc { end }
        | SlideShape::CounterClockwiseArc { end }
        | SlideShape::VShape { end }
        | SlideShape::PShape { end }
        | SlideShape::QShape { end }
        | SlideShape::GrandPShape { end }
        | SlideShape::GrandQShape { end }
        | SlideShape::GrandVShape { end, .. }
        | SlideShape::Thunderbolt { end, .. } => end,
        SlideShape::FanShape {
            ends: (centre, _, _),
        } => centre,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::component::NoteKind;
    use crate::systems::visual::majdata::SlideLayouts;
    use crate::systems::visual::{RADIUS, resources::ButtonLayout};

    /// Every shape the parser can produce, from `start`, for every end key.
    fn shapes_from(start: ButtonId) -> Vec<(SlideShape, ButtonId)> {
        let key = |k: i32| ((k - 1).rem_euclid(8) + 1) as ButtonId;
        let mut out = Vec::new();
        for end in 1..=8 {
            out.push((SlideShape::Straight { end }, end));
            out.push((SlideShape::ShortArc { end }, end));
            out.push((SlideShape::ClockwiseArc { end }, end));
            out.push((SlideShape::CounterClockwiseArc { end }, end));
            out.push((SlideShape::VShape { end }, end));
            out.push((SlideShape::PShape { end }, end));
            out.push((SlideShape::QShape { end }, end));
            out.push((SlideShape::GrandPShape { end }, end));
            out.push((SlideShape::GrandQShape { end }, end));
            out.push((SlideShape::Thunderbolt { end, is_z: false }, end));
            out.push((SlideShape::Thunderbolt { end, is_z: true }, end));
            for mid in [key(start as i32 + 2), key(start as i32 - 2)] {
                out.push((SlideShape::GrandVShape { mid, end }, end));
            }
            let ends = (end, key(end as i32 + 1), key(end as i32 - 1));
            out.push((SlideShape::FanShape { ends }, end));
        }
        out
    }

    /// The key whose ring position is nearest to `p` (pixels).
    fn nearest_key(p: Vec2) -> ButtonId {
        let layout = ButtonLayout::default();
        (1..=8)
            .min_by(|a, b| {
                let da = (layout.tap[a - 1] * RADIUS).distance(p);
                let db = (layout.tap[b - 1] * RADIUS).distance(p);
                da.total_cmp(&db)
            })
            .unwrap()
    }

    fn placed(shape: &SlideShape, start: ButtonId) -> Option<Vec<(Vec2, f32)>> {
        let layouts = SlideLayouts::load();
        let placement = prefab_for(shape, start)?;
        let bars = layouts
            .get(placement.prefab)
            .unwrap_or_else(|| panic!("{shape:?} from {start}: no prefab {}", placement.prefab));
        Some(
            bars.iter()
                .map(|b| place(b, start, placement.mirror))
                .collect(),
        )
    }

    /// Review Focus 1.
    #[test]
    fn every_prefab_the_rules_name_exists() {
        let layouts = SlideLayouts::load();
        for start in 1..=8 {
            for (shape, _) in shapes_from(start) {
                if let Some(p) = prefab_for(&shape, start) {
                    let bars = layouts.get(p.prefab);
                    assert!(
                        bars.is_some_and(|b| b.len() >= 3),
                        "{shape:?} from {start}: {}",
                        p.prefab
                    );
                }
            }
        }
    }

    /// Review Focus 1: the first arrow leaves the start key, the last one
    /// arrives at the end key, from every start, mirrored or not.
    #[test]
    fn slides_land_on_their_end_key() {
        let mut checked = 0;
        for start in 1..=8 {
            for (shape, end) in shapes_from(start) {
                let Some(bars) = placed(&shape, start) else {
                    continue;
                };
                checked += 1;
                assert_eq!(
                    nearest_key(bars[0].0),
                    start,
                    "{shape:?} from {start}: start"
                );
                let last = bars.last().unwrap().0;
                assert_eq!(nearest_key(last), end, "{shape:?} from {start}: end");
            }
        }
        assert!(checked > 300, "{checked}");
    }

    /// Arrows point the way the slide travels: each arrow's facing is close
    /// to the direction to the next one. Catches a wrong mirror or rotation
    /// that still lands on the right keys.
    #[test]
    fn arrows_point_along_the_slide() {
        for start in 1..=8 {
            for (shape, _) in shapes_from(start) {
                if matches!(shape, SlideShape::FanShape { .. }) {
                    continue; // the fan's bars are rows, not arrows
                }
                let Some(bars) = placed(&shape, start) else {
                    continue;
                };
                for pair in bars.windows(2) {
                    // Across a V's tip the arrows turn round; compare arrows
                    // on the same leg only.
                    if (pair[0].1 - pair[1].1).cos() < 0.85 {
                        continue;
                    }
                    let travel = (pair[1].0 - pair[0].0).normalize();
                    let facing = Vec2::from_angle(pair[0].1 + std::f32::consts::PI);
                    assert!(
                        travel.dot(facing) > 0.8,
                        "{shape:?} from {start}: facing {facing:?} travel {travel:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn mirrors_match_majdataplay() {
        let q = prefab_for(&SlideShape::QShape { end: 5 }, 1).unwrap();
        assert_eq!((q.prefab, q.mirror), ("Star_pq_5", true));
        let p = prefab_for(&SlideShape::PShape { end: 5 }, 1).unwrap();
        assert_eq!((p.prefab, p.mirror), ("Star_pq_5", false));
        // `>` from the upper half runs clockwise; from the lower half it is
        // drawn mirrored.
        assert!(
            !prefab_for(&SlideShape::ClockwiseArc { end: 5 }, 1)
                .unwrap()
                .mirror
        );
        assert!(
            prefab_for(&SlideShape::ClockwiseArc { end: 1 }, 5)
                .unwrap()
                .mirror
        );
        // MajdataPlay rejects a straight slide to a neighbouring key.
        assert!(prefab_for(&SlideShape::Straight { end: 2 }, 1).is_none());
    }

    #[test]
    fn the_corpus_has_a_layout_for_every_slide() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../crates/simai/tests/fixtures"
        );
        let mut slides = 0;
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(events) = simai::parse_chart(&text) else {
                continue;
            };
            for event in events {
                let simai::ChartEvent::NoteGroup(notes) = event else {
                    continue;
                };
                for note in notes {
                    let (start, segments) = match &note.kind {
                        NoteKind::Slide {
                            head_button,
                            segments,
                            ..
                        } => (*head_button, segments),
                        NoteKind::HeadlessSlide {
                            start_button,
                            segments,
                            ..
                        } => (*start_button, segments),
                        _ => continue,
                    };
                    let mut from = start;
                    for seg in segments {
                        slides += 1;
                        if prefab_for(&seg.shape, from).is_none() {
                            missing.push(format!("{}: {from} {:?}", path.display(), seg.shape));
                        }
                        from = shape_end(&seg.shape);
                    }
                }
            }
        }
        // Charts the parser accepts and MajdataPlay rejects (they draw no
        // arrows here; the star goes straight to the end key).
        let known_invalid = [
            "slides_simple.txt: 1 VShape { end: 5 }",
            "slides_star.txt: 1 Straight { end: 2 }",
            "slides_star.txt: 6 VShape { end: 2 }",
        ];
        missing.retain(|m| !known_invalid.iter().any(|k| m.ends_with(k)));
        assert!(slides > 0);
        assert!(
            missing.is_empty(),
            "{} of {slides} without a layout:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }
}
