//! Note visual builders.
//!
//! Most note visuals are **textured sprites** loaded from `assets/sprites/` and
//! handed out as cloned [`Handle<Image>`] via [`NoteAssets`]. Only the
//! touch-hold countdown arc stays a lyon vector [`Shape`], because it is redrawn
//! every frame from [`build_countdown_path`].
//!
//! Sprite `custom_size` values are expressed in world units (multiples of
//! [`NOTE_RADIUS`]) so a sprite at `Transform` scale `1.0` occupies the same
//! footprint the old lyon shape did. The leading `SIZE_*` constants are the
//! visual-tuning knobs — adjust these to match the source art, not the spawn
//! sites.

use bevy::prelude::{Color, Handle, Image, Sprite, Vec2};
use bevy_prototype_lyon::prelude::*;
use std::f32::consts::PI;

use crate::systems::visual::{NOTE_RADIUS, RADIUS, resources::NoteAssets};

// ── Sprite footprint constants (world units, tune to the art) ────────────────

const SIZE_TAP: f32 = NOTE_RADIUS * 2.0;
const SIZE_STAR: f32 = NOTE_RADIUS * 2.0;
/// Touch centre dot (`TouchPoint.png`) — small, matching the original dot.
const SIZE_TOUCH: f32 = NOTE_RADIUS * 0.4;
/// Touch "just" hit flash (`TouchJust.png`) — covers the touch hit area.
const SIZE_TOUCH_JUST: f32 = NOTE_RADIUS * 2.0;
/// Touch / TouchHold approach triangle (`Touch_01.png` / `TouchHold_0n.png`,
/// 112×82). Sized so the 4 triangles, once converged just before the hit, span
/// roughly a tap/hold note (~2·NOTE_RADIUS). Height keeps the source aspect.
const TRI_W: f32 = NOTE_RADIUS * 1.5;
const TRI_ASPECT: f32 = 82.0 / 112.0;
/// Hold end caps (`hold_start.jpg` / `hold_end.jpg`, 122×61). Width = beam
/// width; height keeps the 1:2 source ratio. Anchored at the inner edge so each
/// cap sits flush on the beam end (head `BOTTOM_CENTER` at y=0, tail
/// `TOP_CENTER` at y=-len) — set at the spawn site.
const HOLD_CAP_W: f32 = NOTE_RADIUS * 2.0;
const HOLD_CAP_H: f32 = NOTE_RADIUS; // 61/122 * width
/// Hold body beam (`hold.png`, 122×200): full width, **unit height** — Y is
/// scaled at runtime to the live bar length, so the sprite stretches with the
/// hold. Centre-anchored; the mover offsets it by `-len/2`.
const HOLD_BODY_W: f32 = NOTE_RADIUS * 2.0;
const HOLD_BODY_H: f32 = 1.0;
/// Chevron arrow (`Slide.png`, 70×94). Width tracks the wing reach; height keeps
/// the source aspect ratio.
const CHEVRON_ASPECT: f32 = 94.0 / 70.0;
/// The arrow art points left (−X); travel angle is measured from +X. Rotate the
/// spawn transform by this (π) to aim the arrow along the path.
pub(super) const CHEVRON_ROT_OFFSET: f32 = PI;
const SIZE_HEXAGON: f32 = NOTE_RADIUS * 1.5;
const SIZE_HALO: f32 = NOTE_RADIUS * 1.5;
/// Touch death burst (`TouchEff.png`) and its star parts (`TouchEffparts_0n.png`).
const SIZE_TOUCH_EFFECT: f32 = NOTE_RADIUS * 1.0;
const SIZE_TOUCH_EFFECT_STAR: f32 = NOTE_RADIUS * 1.0;
/// Hard-coded tint for the (white) touch burst halo.
const HALO_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
/// `Judge/Normal.png` glow-dot offset from the texture centre, as a fraction of
/// the full sprite size. Measured from the art: the dot sits at y ≈ 5.46% from
/// the top, i.e. 0.5 − 0.0546. The dot lies on the ring's top edge, so this is
/// simultaneously (a) the anchor that pins the dot to the note centre and (b)
/// the ring's radius as a fraction of the sprite — used to size the sprite so a
/// `Transform` scale of 1.0 makes the ring coincide with the judgement circle.
pub(super) const NORMAL_HALO_DOT_OFFSET: f32 = 0.4454;
/// Note halo ring (`Judge/Normal.png`) drawn behind tap/star/hold. Sized so the
/// ring's radius (`NORMAL_HALO_DOT_OFFSET` · size) equals `RADIUS` at scale 1.0.
const SIZE_NORMAL_HALO: f32 = RADIUS / NORMAL_HALO_DOT_OFFSET;
/// Hold tail glow dot (`Hold_End_dot.png`).
const SIZE_HOLD_END_DOT: f32 = NOTE_RADIUS * 0.8;

// ── Sprite builders ──────────────────────────────────────────────────────────

fn sprite(image: bevy::prelude::Handle<bevy::prelude::Image>, size: Vec2) -> Sprite {
    Sprite {
        image,
        custom_size: Some(size),
        ..Default::default()
    }
}

/// Tap note (`sprites/Tap/tap.png`). Untinted — the art carries its own colour.
pub(super) fn tap_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.tap.clone(), Vec2::splat(SIZE_TAP))
}

/// Tap break variant (`tap_break.png`), same footprint as `tap_sprite`.
pub(super) fn tap_break_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.tap_break.clone(), Vec2::splat(SIZE_TAP))
}

/// Tap EX outline overlay (`tap_ex.png`), drawn on top of the note.
pub(super) fn tap_ex_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.tap_ex.clone(), Vec2::splat(SIZE_TAP))
}

/// Slide / spark star (`sprites/Slide/Star.png`), tinted per note colour.
pub(super) fn star_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.star.clone(), Vec2::splat(SIZE_STAR))
}

/// Star break variant (`star_break.png`), same footprint as `star_sprite`.
pub(super) fn star_break_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.star_break.clone(), Vec2::splat(SIZE_STAR))
}

/// Star EX outline overlay (`star_ex.png`), drawn on top of the star.
pub(super) fn star_ex_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.star_ex.clone(), Vec2::splat(SIZE_STAR))
}

/// Hold head/tail cap (`hold_*_start/end.png`). The caller passes the variant
/// handle (normal / break / ex) and sets the anchor at the spawn site.
pub(super) fn hold_cap_sprite(image: Handle<Image>) -> Sprite {
    sprite(image, Vec2::new(HOLD_CAP_W, HOLD_CAP_H))
}

/// Hold body beam (`hold_*_mid.png`), unit height; Y scaled at runtime. The
/// caller passes the variant handle (normal / break / ex).
pub(super) fn hold_beam_sprite(image: Handle<Image>) -> Sprite {
    sprite(image, Vec2::new(HOLD_BODY_W, HOLD_BODY_H))
}

/// Touch note centre dot (`sprites/Touch/TouchPoint.png`), tinted per colour.
pub(super) fn touch_circle_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.touch.clone(), Vec2::splat(SIZE_TOUCH))
}

/// Touch "just" hit flash (`sprites/Touch/TouchJust.png`), shown briefly at the
/// moment a touch is struck. Sized to cover the converged touch hit area.
pub(super) fn touch_just_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.touch_just.clone(), Vec2::splat(SIZE_TOUCH_JUST))
}

/// Touch approach triangle (`Touch_01.png`, points +Y). Rotated to its
/// direction at the spawn site; rendered with the art's own colour.
pub(super) fn touch_triangle_sprite(assets: &NoteAssets) -> Sprite {
    sprite(
        assets.touch_triangle.clone(),
        Vec2::new(TRI_W, TRI_W * TRI_ASPECT),
    )
}

/// TouchHold directional triangle `idx` (0..4 → `TouchHold_01..04.png`, points
/// +Y). The art is pre-coloured per direction, so it renders untinted.
pub(super) fn touch_hold_triangle_sprite(assets: &NoteAssets, idx: usize) -> Sprite {
    sprite(
        assets.touch_hold[idx].clone(),
        Vec2::new(TRI_W, TRI_W * TRI_ASPECT),
    )
}

/// Slide-track chevron (`sprites/Slide/Slide.png`). `radius` scales the wings,
/// `thickness` the stem; spawned transparent and faded in via `Sprite.color`.
pub(super) fn chevron_sprite(assets: &NoteAssets, radius: f32, _thickness: f32) -> Sprite {
    let w = radius;
    sprite(assets.chevron.clone(), Vec2::new(w, w * CHEVRON_ASPECT))
}

/// Lyon vector chevron for the **fan / wifi** cone. Unlike the sprite, the wings
/// scale with `radius` while the stem `thickness` stays fixed, so the widening
/// cone stays crisp instead of stretching the texture. Tip points +X, so the
/// spawn rotation is the bare travel angle (no `CHEVRON_ROT_OFFSET`).
pub(super) fn chevron_shape(radius: f32, thickness: f32, color: Color) -> Shape {
    ShapeBuilder::with(&build_chevron_path(radius, thickness))
        .fill(color)
        .build()
}

fn build_chevron_path(radius: f32, thickness: f32) -> ShapePath {
    let half_angle = 3.0 * PI / 8.0;
    let depth = radius * half_angle.cos();
    let height = radius * half_angle.sin();
    let inner_x = thickness / 2.0;
    let outer_x = -thickness / 2.0;

    ShapePath::new()
        .move_to(Vec2::new(-depth + inner_x, height))
        .line_to(Vec2::new(inner_x, 0.0))
        .line_to(Vec2::new(-depth + inner_x, -height))
        .line_to(Vec2::new(-depth + outer_x, -height))
        .line_to(Vec2::new(outer_x, 0.0))
        .line_to(Vec2::new(-depth + outer_x, height))
        .close()
}

/// Hexagon pop effect (`sprites/Effect/Hex.png`), tinted.
pub(super) fn hexagon_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.hexagon.clone(), Vec2::splat(SIZE_HEXAGON))
}

/// Expanding hold / touch halo (`sprites/Effect/Circle.png`), tinted.
/// Touch death burst halo (`sprites/Touch/TouchEff.png`) — replaces the generic
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

/// Glow dot pinned to the hold tail end (`Hold_End_dot.png`).
pub(super) fn hold_end_dot_sprite(assets: &NoteAssets) -> Sprite {
    sprite(assets.hold_end_dot.clone(), Vec2::splat(SIZE_HOLD_END_DOT))
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
