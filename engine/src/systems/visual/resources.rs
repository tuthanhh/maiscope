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
    // ── Sprite textures: MajdataPlay's (`majdata/`), visualizer-majdata 15 ──
    /// Tap note and its break / each variants; `tap_ex` is the EX outline
    /// drawn on top (`skin/TapSkins/`).
    pub tap: Handle<Image>,
    pub tap_break: Handle<Image>,
    pub tap_each: Handle<Image>,
    pub tap_ex: Handle<Image>,
    /// Star (a slide's head, `$`) and its variants; `_double` is a star that
    /// leads more than one slide (`skin/StarSkins/`).
    pub star: Handle<Image>,
    pub star_break: Handle<Image>,
    pub star_each: Handle<Image>,
    pub star_ex: Handle<Image>,
    pub star_double: Handle<Image>,
    pub star_break_double: Handle<Image>,
    pub star_each_double: Handle<Image>,
    /// Hold: one image per variant, cut into head / body / tail with
    /// `Sprite.rect` (`skin/HoldSkins/`); `hold_ex` is overlaid like `tap_ex`.
    pub hold: Handle<Image>,
    pub hold_break: Handle<Image>,
    pub hold_each: Handle<Image>,
    pub hold_ex: Handle<Image>,
    /// Glow dot pinned to the hold tail end (`skin/NoteGuideSkins/Hold_End.png`).
    pub hold_end_dot: Handle<Image>,
    /// Note guide ring + glow dot (`skin/NoteGuideSkins/Normal.png`).
    pub normal_halo: Handle<Image>,
    /// Touch note centre dot (`skin/TouchSkins/touch_point.png`).
    pub touch: Handle<Image>,
    /// Touch "just" hit flash (`skin/TouchSkins/touch_just.png`).
    pub touch_just: Handle<Image>,
    /// Touch approach petal (`skin/TouchSkins/touch.png`), rotated into the 4
    /// directions at spawn.
    pub touch_triangle: Handle<Image>,
    /// TouchHold petals, one per direction (`skin/TouchHoldSkins/touchhold_0..3`).
    pub touch_hold: [Handle<Image>; 4],
    /// Touch death burst (`effects/TouchEff.png`) and its star parts
    /// (`effects/TouchEffparts_01/02.png`).
    pub touch_effect: Handle<Image>,
    pub touch_effect_stars: [Handle<Image>; 2],
    /// Slide arrow: normal, each, break (`skin/SlideSkins/`).
    pub slide: [Handle<Image>; 3],
    /// A fan's rows `wifi_0..10`: normal, each, break (`skin/WifiSkins/`).
    pub wifi: [[Handle<Image>; 11]; 3],
    /// Hexagon pop effect (`effects/Hex.png`).
    pub hexagon: Handle<Image>,
    /// Judgement ring with its eight key marks (`skin/outline.png`).
    pub ring: Handle<Image>,
    /// Expanding hold / touch halo (`sprites/Effect/Circle.png`): the one
    /// sprite MajdataPlay has no counterpart for.
    pub halo: Handle<Image>,
}

impl FromWorld for NoteAssets {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        let m = |path: &str| server.load(format!("majdata/{path}"));
        Self {
            tap: m("skin/TapSkins/tap.png"),
            tap_break: m("skin/TapSkins/tap_break.png"),
            tap_each: m("skin/TapSkins/tap_each.png"),
            tap_ex: m("skin/TapSkins/tap_ex.png"),
            star: m("skin/StarSkins/star.png"),
            star_break: m("skin/StarSkins/star_break.png"),
            star_each: m("skin/StarSkins/star_each.png"),
            star_ex: m("skin/StarSkins/star_ex.png"),
            star_double: m("skin/StarSkins/star_double.png"),
            star_break_double: m("skin/StarSkins/star_break_double.png"),
            star_each_double: m("skin/StarSkins/star_each_double.png"),
            hold: m("skin/HoldSkins/hold.png"),
            hold_break: m("skin/HoldSkins/hold_break.png"),
            hold_each: m("skin/HoldSkins/hold_each.png"),
            hold_ex: m("skin/HoldSkins/hold_ex.png"),
            hold_end_dot: m("skin/NoteGuideSkins/Hold_End.png"),
            normal_halo: m("skin/NoteGuideSkins/Normal.png"),
            touch: m("skin/TouchSkins/touch_point.png"),
            touch_just: m("skin/TouchSkins/touch_just.png"),
            touch_triangle: m("skin/TouchSkins/touch.png"),
            touch_hold: std::array::from_fn(|i| {
                m(&format!("skin/TouchHoldSkins/touchhold_{i}.png"))
            }),
            touch_effect: m("effects/TouchEff.png"),
            touch_effect_stars: [
                m("effects/TouchEffparts_01.png"),
                m("effects/TouchEffparts_02.png"),
            ],
            slide: ["slide", "slide_each", "slide_break"]
                .map(|name| m(&format!("skin/SlideSkins/{name}.png"))),
            wifi: ["", "each_", "break_"].map(|variant| {
                std::array::from_fn(|row| m(&format!("skin/WifiSkins/wifi_{variant}{row}.png")))
            }),
            hexagon: m("effects/Hex.png"),
            ring: m("skin/outline.png"),
            halo: server.load("sprites/Effect/Circle.png"),
        }
    }
}
