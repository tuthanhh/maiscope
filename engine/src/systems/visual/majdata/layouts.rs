//! `slides.txt` and `sprites.txt`, embedded at compile time and parsed once.

use bevy::prelude::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::systems::visual::RADIUS;

const SLIDES: &str = include_str!("../../../../assets/majdata/slides.txt");
const SPRITES: &str = include_str!("../../../../assets/majdata/sprites.txt");

/// MajdataPlay's ring is 4.8 world units from the centre and its skin is 100
/// pixels to the unit; ours is `RADIUS` pixels. One of its pixels is this
/// many of ours.
pub const SKIN_SCALE: f32 = RADIUS / 480.0;

/// One arrow of a slide prefab, as MajdataPlay lays it out for a slide that
/// starts at key 1: its position in MajdataPlay world units, and its angle.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub name: String,
    pub pos: Vec2,
    pub angle_deg: f32,
}

/// Every slide prefab's arrows, in draw order (the order the star walks).
#[derive(Resource, Debug, Clone)]
pub struct SlideLayouts(HashMap<String, Vec<Bar>>);

impl SlideLayouts {
    pub fn load() -> Self {
        let mut prefabs: HashMap<String, Vec<Bar>> = HashMap::new();
        let mut current: Option<String> = None;
        for line in SLIDES.lines() {
            if let Some(name) = line.strip_prefix("prefab ") {
                let name = name.trim().to_string();
                prefabs.entry(name.clone()).or_default();
                current = Some(name);
            } else if let Some(rest) = line.strip_prefix("bar ") {
                // Unity names may hold spaces (`Slide_01 (21)`): the three
                // numbers are read from the end, the name is what is left.
                let mut words = rest.trim().rsplitn(4, char::is_whitespace);
                let mut number = || -> f32 {
                    words
                        .next()
                        .and_then(|w| w.parse().ok())
                        .expect("bar number")
                };
                let angle_deg = number();
                let y = number();
                let x = number();
                let name = words.next().expect("bar name").trim().to_string();
                {
                    let prefab = current.as_ref().expect("bar before any prefab");
                    prefabs.get_mut(prefab).unwrap().push(Bar {
                        name,
                        pos: Vec2::new(x, y),
                        angle_deg,
                    });
                }
            }
        }
        Self(prefabs)
    }

    pub fn get(&self, prefab: &str) -> Option<&[Bar]> {
        self.0.get(prefab).map(Vec::as_slice)
    }
}

impl Default for SlideLayouts {
    fn default() -> Self {
        Self::load()
    }
}

static SIZES: Lazy<HashMap<&'static str, Vec2>> = Lazy::new(|| {
    SPRITES
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let path = words.next()?;
            let w: f32 = words.next()?.parse().ok()?;
            let h: f32 = words.next()?.parse().ok()?;
            Some((path, Vec2::new(w, h)))
        })
        .collect()
});

/// A MajdataPlay sprite's pixel size. `path` is relative to
/// `engine/assets/majdata/`.
pub fn sprite_pixels(path: &str) -> Vec2 {
    let size = SIZES.get(path).copied();
    debug_assert!(size.is_some(), "no MajdataPlay sprite {path}");
    size.unwrap_or(Vec2::splat(100.0))
}

/// A MajdataPlay sprite's size on our playfield, from its pixel size, so the
/// art keeps MajdataPlay's proportions to the ring.
pub fn sprite_px(path: &str) -> Vec2 {
    sprite_pixels(path) * SKIN_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::visual::RADIUS;

    #[test]
    fn the_layouts_load() {
        let layouts = SlideLayouts::load();
        let line = layouts.get("Star_Line_5").expect("Star_Line_5");
        assert!(line.len() > 3);
        assert_eq!(layouts.get("Slide_Wifi").map(|b| b.len()), Some(11));
        assert!(layouts.get("Star_Nothing").is_none());
    }

    #[test]
    fn sprite_sizes_follow_majdataplay_scale() {
        let close = |a: Vec2, b: Vec2| (a - b).length() < 1e-3;
        assert!(close(
            sprite_px("skin/TapSkins/tap.png"),
            Vec2::splat(122.0 * RADIUS / 480.0)
        ));
        assert!(close(
            sprite_px("skin/SlideSkins/slide.png"),
            Vec2::new(70.0, 94.0) * RADIUS / 480.0
        ));
    }
}
