use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct ButtonLayout {
    pub tap: Vec<Vec2>,
    pub a: Vec<Vec2>,
    pub b: Vec<Vec2>,
    pub c: Vec<Vec2>,
    pub d: Vec<Vec2>,
    pub e: Vec<Vec2>,
    pub tap_spawn: Vec<Vec2>,
}

impl Default for ButtonLayout {
    fn default() -> Self {
        let mut tap = Vec::new();
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut c = Vec::new();
        let mut d = Vec::new();
        let mut e = Vec::new();
        let mut tap_spawn = Vec::new();
        for i in 0..8 {
            let a1 = std::f32::consts::FRAC_PI_2
                - std::f32::consts::FRAC_PI_8
                - (i as f32) * std::f32::consts::FRAC_PI_4;
            let a2 = std::f32::consts::FRAC_PI_2 - (i as f32) * std::f32::consts::FRAC_PI_4;
            tap.push(Vec2::new(1.0 * a1.cos(), 1.0 * a1.sin()));
            a.push(Vec2::new(4.1 / 4.8 * a1.cos(), 4.1 / 4.8 * a1.sin()));
            d.push(Vec2::new(4.1 / 4.8 * a2.cos(), 4.1 / 4.8 * a2.sin()));
            b.push(Vec2::new(2.3 / 4.8 * a1.cos(), 2.3 / 4.8 * a1.sin()));
            e.push(Vec2::new(3.0 / 4.8 * a2.cos(), 3.0 / 4.8 * a2.sin()));
            tap_spawn.push(Vec2::new(1.225 / 4.8 * a1.cos(), 1.225 / 4.8 * a1.sin()));
        }
        for _ in 0..3 {
            c.push(Vec2::ZERO);
        }

        Self {
            tap,
            a,
            b,
            c,
            d,
            e,
            tap_spawn,
        }
    }
}

/// Shared visual assets for all note types.
///
/// Most fields are sprite [`Handle<Image>`]s, loaded once and cloned at every
/// spawn (the `AssetServer` dedupes by path, so cloning never re-reads the
/// file). A strong handle is held here for the whole session so textures are
/// never unloaded between note waves. The remaining [`ShapePath`] fields are
/// the lyon visuals with no sprite equivalent (approach triangles, countdown).
#[derive(Resource)]
pub struct NoteAssets {
    // ── Sprite textures ────────────────────────────────────────────────
    /// Tap note (`sprites/Tap/tap.png`).
    pub tap: Handle<Image>,
    /// Tap break variant (`sprites/Tap/tap_break.png`) — replaces `tap`.
    pub tap_break: Handle<Image>,
    /// Tap EX outline overlay (`sprites/Tap/tap_ex.png`) — drawn on top.
    pub tap_ex: Handle<Image>,
    /// Slide / spark star (`sprites/Slide/Star.png`).
    pub star: Handle<Image>,
    /// Star break variant (`sprites/Slide/star_break.png`) — replaces `star`.
    pub star_break: Handle<Image>,
    /// Star EX outline overlay (`sprites/Slide/star_ex.png`) — drawn on top.
    pub star_ex: Handle<Image>,
    /// Hold head cap — `^` (`sprites/Hold/hold_start.jpg`).
    pub hold_head: Handle<Image>,
    /// Hold body rails, stretched in Y (`sprites/Hold/hold_mid.jpg`).
    pub hold_body: Handle<Image>,
    /// Hold tail cap — `v` (`sprites/Hold/hold_end.jpg`).
    pub hold_tail: Handle<Image>,
    /// Hold break slices — replace the normal head/body/tail.
    pub hold_break_head: Handle<Image>,
    pub hold_break_body: Handle<Image>,
    pub hold_break_tail: Handle<Image>,
    /// Hold EX outline slices — overlaid on top of the base hold.
    pub hold_ex_head: Handle<Image>,
    pub hold_ex_body: Handle<Image>,
    pub hold_ex_tail: Handle<Image>,
    /// Glow dot pinned to the hold tail end (`sprites/Hold/Hold_End_dot.png`).
    pub hold_end_dot: Handle<Image>,
    /// Note halo ring + glow dot (`sprites/Judge/Normal.png`) for tap/star/hold.
    pub normal_halo: Handle<Image>,
    /// Touch note centre dot (`sprites/Touch/TouchPoint.png`).
    pub touch: Handle<Image>,
    /// Touch "just" hit flash (`sprites/Touch/TouchJust.png`), shown for the
    /// brief Holding phase when a touch is struck.
    pub touch_just: Handle<Image>,
    /// Touch approach triangle (`sprites/Touch/Touch_01.png`, points +Y),
    /// rotated into the 4 directions at spawn.
    pub touch_triangle: Handle<Image>,
    /// TouchHold directional triangles (`sprites/Touch/TouchHold_01..04.png`),
    /// one coloured sprite per direction.
    pub touch_hold: [Handle<Image>; 4],
    /// Touch death burst halo (`sprites/Touch/TouchEff.png`).
    pub touch_effect: Handle<Image>,
    /// Touch burst star parts (`TouchEffparts_01/02.png`), alternated across the
    /// 8 converging stars.
    pub touch_effect_stars: [Handle<Image>; 2],
    /// Slide-track chevron arrow (`sprites/Slide/Slide.png`).
    pub chevron: Handle<Image>,
    /// Hexagon pop effect (`sprites/Effect/Hex.png`).
    pub hexagon: Handle<Image>,
    /// Expanding hold / touch halo (`sprites/Effect/Circle.png`).
    pub halo: Handle<Image>,
}

impl FromWorld for NoteAssets {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        Self {
            tap: server.load("sprites/Tap/tap.png"),
            tap_break: server.load("sprites/Tap/tap_break.png"),
            tap_ex: server.load("sprites/Tap/tap_ex.png"),
            star: server.load("sprites/Slide/Star.png"),
            star_break: server.load("sprites/Slide/star_break.png"),
            star_ex: server.load("sprites/Slide/star_ex.png"),
            hold_head: server.load("sprites/Hold/hold_start.png"),
            hold_body: server.load("sprites/Hold/hold_mid.png"),
            hold_tail: server.load("sprites/Hold/hold_end.png"),
            hold_break_head: server.load("sprites/Hold/hold_break_start.png"),
            hold_break_body: server.load("sprites/Hold/hold_break_mid.png"),
            hold_break_tail: server.load("sprites/Hold/hold_break_end.png"),
            hold_ex_head: server.load("sprites/Hold/hold_ex_start.png"),
            hold_ex_body: server.load("sprites/Hold/hold_ex_mid.png"),
            hold_ex_tail: server.load("sprites/Hold/hold_ex_end.png"),
            hold_end_dot: server.load("sprites/Hold/Hold_End_dot.png"),
            normal_halo: server.load("sprites/Judge/Normal.png"),
            touch: server.load("sprites/Touch/TouchPoint.png"),
            touch_just: server.load("sprites/Touch/TouchJust.png"),
            touch_triangle: server.load("sprites/Touch/Touch_01.png"),
            touch_hold: [
                server.load("sprites/Touch/TouchHold_01.png"),
                server.load("sprites/Touch/TouchHold_02.png"),
                server.load("sprites/Touch/TouchHold_03.png"),
                server.load("sprites/Touch/TouchHold_04.png"),
            ],
            touch_effect: server.load("sprites/Touch/TouchEff.png"),
            touch_effect_stars: [
                server.load("sprites/Touch/TouchEffparts_01.png"),
                server.load("sprites/Touch/TouchEffparts_02.png"),
            ],
            chevron: server.load("sprites/Slide/Slide.png"),
            hexagon: server.load("sprites/Effect/Hex.png"),
            halo: server.load("sprites/Effect/Circle.png"),
        }
    }
}
