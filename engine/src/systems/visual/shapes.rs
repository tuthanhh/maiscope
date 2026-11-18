//! Note visual builders.
//!
//! Most note visuals are **textured sprites** — MajdataPlay's, from
//! `assets/majdata/` (visualizer-majdata 15) — and
//! handed out as cloned [`Handle<Image>`] via [`NoteAssets`]. Only the
//! touch-hold countdown arc stays a lyon vector [`Shape`], because it is redrawn
//! every frame from [`build_countdown_path`].
//!
//! Sprite `custom_size` values are expressed in world units (multiples of
//! [`NOTE_RADIUS`]) so a sprite at `Transform` scale `1.0` occupies the same
//! footprint the old lyon shape did. The leading `SIZE_*` constants are the
//! visual-tuning knobs — adjust these to match the source art, not the spawn
//! sites.

use bevy::prelude::{Color, Handle, Image, Rect, Sprite, Vec2};
use bevy_prototype_lyon::prelude::*;

use super::component::HoldNoteElement;
use super::majdata::{SKIN_SCALE, sprite_pixels, sprite_px};
use crate::systems::visual::{NOTE_RADIUS, RADIUS, resources::NoteAssets};

// ── Sprite sizes ─────────────────────────────────────────────────────────────
// Note art is MajdataPlay's (visualizer-majdata 15) and keeps its proportions
// to the ring: each sprite's size is its pixel size × `SKIN_SCALE`
// (`majdata::sprite_px`). The effects keep the sizes below.
const SIZE_HEXAGON: f32 = NOTE_RADIUS * 1.5;
const SIZE_HALO: f32 = NOTE_RADIUS * 1.5;
/// Touch death burst (`TouchEff.png`) and its star parts (`TouchEffparts_0n.png`).
const SIZE_TOUCH_EFFECT: f32 = NOTE_RADIUS * 1.0;
const SIZE_TOUCH_EFFECT_STAR: f32 = NOTE_RADIUS * 1.0;
/// Hard-coded tint for the (white) touch burst halo.
const HALO_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
/// `NoteGuideSkins/Normal.png` glow-dot offset from the texture centre, as a
/// fraction of the full sprite size. Measured from the art: the dot sits at
/// y ≈ 5.46% from the top, i.e. 0.5 − 0.0546. The dot lies on the ring's top
/// edge, so this is simultaneously (a) the anchor that pins the dot to the note
/// centre and (b) the ring's radius as a fraction of the sprite — used to size
/// the sprite so a `Transform` scale of 1.0 makes the ring coincide with the
/// judgement circle.
pub(super) const NORMAL_HALO_DOT_OFFSET: f32 = 0.4454;
/// Note halo ring sized so the ring's radius (`NORMAL_HALO_DOT_OFFSET` · size)
/// equals `RADIUS` at scale 1.0.
const SIZE_NORMAL_HALO: f32 = RADIUS / NORMAL_HALO_DOT_OFFSET;
/// The caps of MajdataPlay's hold image, top (head) and bottom (tail), in its
/// pixels; the rows between them stretch with the hold.
const HOLD_CAP_PX: f32 = 58.0;

// ── Sprite builders ──────────────────────────────────────────────────────────

fn sprite(image: bevy::prelude::Handle<bevy::prelude::Image>, size: Vec2) -> Sprite {
    Sprite {
        image,
        custom_size: Some(size),
        ..Default::default()
    }
}

/// A MajdataPlay sprite at its own size on our playfield.
fn skin(image: Handle<Image>, path: &str) -> Sprite {
    sprite(image, sprite_px(path))
}

/// Which variant of a note's art to draw. Break wins over each, as in
/// MajdataPlay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Art {
    Normal,
    Each,
    Break,
}

/// Tap note (`skin/TapSkins/tap*.png`). Untinted — the art carries its colour.
pub(super) fn tap_sprite(assets: &NoteAssets, art: Art) -> Sprite {
    match art {
        Art::Normal => skin(assets.tap.clone(), "skin/TapSkins/tap.png"),
        Art::Each => skin(assets.tap_each.clone(), "skin/TapSkins/tap_each.png"),
        Art::Break => skin(assets.tap_break.clone(), "skin/TapSkins/tap_break.png"),
    }
}

/// Tap EX outline overlay (`tap_ex.png`), drawn on top of the note.
pub(super) fn tap_ex_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.tap_ex.clone(), "skin/TapSkins/tap_ex.png")
}

/// Star (`skin/StarSkins/star*.png`): a slide's head or a `$` star. `double`:
/// it leads more than one slide.
pub(super) fn star_sprite(assets: &NoteAssets, art: Art, double: bool) -> Sprite {
    match (art, double) {
        (Art::Normal, false) => skin(assets.star.clone(), "skin/StarSkins/star.png"),
        (Art::Each, false) => skin(assets.star_each.clone(), "skin/StarSkins/star_each.png"),
        (Art::Break, false) => skin(assets.star_break.clone(), "skin/StarSkins/star_break.png"),
        (Art::Normal, true) => skin(assets.star_double.clone(), "skin/StarSkins/star_double.png"),
        (Art::Each, true) => skin(
            assets.star_each_double.clone(),
            "skin/StarSkins/star_each_double.png",
        ),
        (Art::Break, true) => skin(
            assets.star_break_double.clone(),
            "skin/StarSkins/star_break_double.png",
        ),
    }
}

/// Star EX outline overlay (`star_ex.png`), drawn on top of the star.
pub(super) fn star_ex_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.star_ex.clone(), "skin/StarSkins/star_ex.png")
}

/// The image and file of a hold's art (`skin/HoldSkins/`); `ex` is the EX
/// overlay.
pub(super) fn hold_art(assets: &NoteAssets, art: Art, ex: bool) -> (Handle<Image>, &'static str) {
    match (art, ex) {
        (_, true) => (assets.hold_ex.clone(), "skin/HoldSkins/hold_ex.png"),
        (Art::Normal, _) => (assets.hold.clone(), "skin/HoldSkins/hold.png"),
        (Art::Each, _) => (assets.hold_each.clone(), "skin/HoldSkins/hold_each.png"),
        (Art::Break, _) => (assets.hold_break.clone(), "skin/HoldSkins/hold_break.png"),
    }
}

/// One part of a hold, cut out of its single MajdataPlay image with
/// `Sprite.rect`: the head (top cap), the body (the rows between the caps,
/// unit height — Y is scaled at runtime to the live bar length) or the tail
/// (bottom cap).
pub(super) fn hold_part_sprite(image: Handle<Image>, path: &str, part: &HoldNoteElement) -> Sprite {
    let px = sprite_pixels(path);
    let width = px.x * SKIN_SCALE;
    let cap = HOLD_CAP_PX * SKIN_SCALE;
    let (rect, size) = match part {
        HoldNoteElement::Head => (
            Rect::new(0.0, 0.0, px.x, HOLD_CAP_PX),
            Vec2::new(width, cap),
        ),
        HoldNoteElement::Tail => (
            Rect::new(0.0, px.y - HOLD_CAP_PX, px.x, px.y),
            Vec2::new(width, cap),
        ),
        _ => (
            Rect::new(0.0, HOLD_CAP_PX, px.x, px.y - HOLD_CAP_PX),
            Vec2::new(width, 1.0),
        ),
    };
    Sprite {
        rect: Some(rect),
        ..sprite(image, size)
    }
}

/// Touch note centre dot (`skin/TouchSkins/touch_point.png`).
pub(super) fn touch_circle_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.touch.clone(), "skin/TouchSkins/touch_point.png")
}

/// Touch "just" hit flash (`skin/TouchSkins/touch_just.png`), shown briefly at
/// the moment a touch is struck.
pub(super) fn touch_just_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.touch_just.clone(), "skin/TouchSkins/touch_just.png")
}

/// Touch approach petal (`skin/TouchSkins/touch.png`, points +Y). Rotated to
/// its direction at the spawn site; rendered with the art's own colour.
pub(super) fn touch_triangle_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.touch_triangle.clone(), "skin/TouchSkins/touch.png")
}

/// TouchHold petal `idx` (0..4 → `touchhold_0..3.png`, points +Y). The art is
/// pre-coloured per direction, so it renders untinted.
pub(super) fn touch_hold_triangle_sprite(assets: &NoteAssets, idx: usize) -> Sprite {
    skin(
        assets.touch_hold[idx].clone(),
        &format!("skin/TouchHoldSkins/touchhold_{idx}.png"),
    )
}

/// Hexagon pop effect (`effects/Hex.png`), tinted.
pub(super) fn hexagon_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.hexagon.clone(), Vec2::splat(SIZE_HEXAGON))
}

/// Expanding hold / touch halo (`sprites/Effect/Circle.png`), tinted.
/// Touch death burst halo (`effects/TouchEff.png`) — replaces the generic
/// `Circle.png` halo; tinted yellow and scaled + faded by `animate_touch_spark`.
pub(super) fn touch_effect_sprite(assets: &NoteAssets) -> Sprite {
    Sprite {
        image: assets.touch_effect.clone(),
        custom_size: Some(Vec2::splat(SIZE_TOUCH_EFFECT)),
        color: HALO_COLOR,
        ..Default::default()
    }
}

/// Touch burst star part `idx` — alternates `TouchEffparts_01/02.png`.
pub(super) fn touch_effect_star_sprite(assets: &NoteAssets, idx: usize) -> Sprite {
    Sprite {
        image: assets.touch_effect_stars[idx % 2].clone(),
        custom_size: Some(Vec2::splat(SIZE_TOUCH_EFFECT_STAR)),
        color: HALO_COLOR,
        ..Default::default()
    }
}

/// Note halo (`Judge/Normal.png`): a faint ring with a glow dot, drawn behind
/// the note. Centred on tap/star; bottom-centre-anchored at the hold head.
pub(super) fn normal_halo_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.normal_halo.clone(), Vec2::splat(SIZE_NORMAL_HALO))
}

/// Glow dot pinned to the hold tail end (`skin/NoteGuideSkins/Hold_End.png`).
pub(super) fn hold_end_dot_sprite(assets: &NoteAssets) -> Sprite {
    skin(
        assets.hold_end_dot.clone(),
        "skin/NoteGuideSkins/Hold_End.png",
    )
}

/// The judgement ring with its eight key marks (`skin/outline.png`).
pub(super) fn ring_sprite(assets: &NoteAssets) -> Sprite {
    skin(assets.ring.clone(), "skin/outline.png")
}

pub(super) fn hold_halo_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.halo.clone(), Vec2::splat(SIZE_HALO))
}

/// The start distance used when placing approach triangles.
/// Exposed as a function so `spawning.rs` and `movement.rs` always agree.
pub fn touch_triangle_start_distance(note_radius: f32) -> f32 {
    0.65 * note_radius
}

/// Half-diagonal of the touch-hold countdown diamond.
pub(super) const COUNTDOWN_SWEEP_S: f32 = NOTE_RADIUS * 1.1;
/// Number of coloured edges making up the countdown diamond.
pub(super) const COUNTDOWN_EDGES: usize = 4;
/// Per-edge colours of the touch-hold border, in clockwise order from the top
/// corner (top→right, right→bottom, bottom→left, left→top).
pub(super) const COUNTDOWN_EDGE_COLORS: [Color; COUNTDOWN_EDGES] = [
    Color::srgb(0.95, 0.25, 0.25), // red
    Color::srgb(0.98, 0.82, 0.18), // yellow
    Color::srgb(0.30, 0.80, 0.35), // green
    Color::srgb(0.25, 0.55, 0.95), // blue
];

/// Diamond corners, clockwise from the top.
fn countdown_corner(i: usize, s: f32) -> Vec2 {
    match i % COUNTDOWN_EDGES {
        0 => Vec2::new(0.0, s),
        1 => Vec2::new(s, 0.0),
        2 => Vec2::new(0.0, -s),
        _ => Vec2::new(-s, 0.0),
    }
}

/// Partial path for countdown `edge` (0..4), drawn `frac` of the way from its
/// start corner toward the next — the per-edge piece of the clockwise reveal.
pub(super) fn countdown_edge(edge: usize, s: f32, frac: f32) -> ShapePath {
    let from = countdown_corner(edge, s);
    let to = countdown_corner(edge + 1, s);
    ShapePath::new()
        .move_to(from)
        .line_to(from.lerp(to, frac.clamp(0.0, 1.0)))
}
