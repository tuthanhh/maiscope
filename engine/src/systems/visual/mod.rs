mod component;
mod judgement_circle;
mod movement;
pub mod resources;
mod shapes;
mod slide_path;
mod spawning;

pub use component::NoteTiming;
pub use judgement_circle::spawn_judgement_ring;
pub use movement::update_movement;
pub use spawning::{apply_commands, next_event};

/// One-shot diagnostic: logs the load state of every sprite handle until all are
/// resolved, so a missing texture is obvious in the console (browser DevTools on
/// wasm). `Failed` ⇒ 404 / wrong path (serving issue); `Loaded` ⇒ the file is
/// fine and any invisibility is a size/anchor/z problem. Not scheduled by
/// default; add to `Update` temporarily when debugging asset loads.
#[allow(dead_code)]
pub fn report_sprite_loads(
    server: bevy::prelude::Res<bevy::prelude::AssetServer>,
    assets: bevy::prelude::Res<resources::NoteAssets>,
    mut done: bevy::prelude::Local<bool>,
) {
    use bevy::asset::LoadState;
    use bevy::prelude::*;

    if *done {
        return;
    }

    let handles: [(&str, &Handle<Image>); 30] = [
        ("tap", &assets.tap),
        ("tap_break", &assets.tap_break),
        ("tap_ex", &assets.tap_ex),
        ("star", &assets.star),
        ("star_break", &assets.star_break),
        ("star_ex", &assets.star_ex),
        ("hold_head", &assets.hold_head),
        ("hold_body", &assets.hold_body),
        ("hold_tail", &assets.hold_tail),
        ("hold_break_head", &assets.hold_break_head),
        ("hold_break_body", &assets.hold_break_body),
        ("hold_break_tail", &assets.hold_break_tail),
        ("hold_ex_head", &assets.hold_ex_head),
        ("hold_ex_body", &assets.hold_ex_body),
        ("hold_ex_tail", &assets.hold_ex_tail),
        ("hold_end_dot", &assets.hold_end_dot),
        ("normal_halo", &assets.normal_halo),
        ("touch", &assets.touch),
        ("touch_just", &assets.touch_just),
        ("touch_triangle", &assets.touch_triangle),
        ("touch_hold_0", &assets.touch_hold[0]),
        ("touch_hold_1", &assets.touch_hold[1]),
        ("touch_hold_2", &assets.touch_hold[2]),
        ("touch_hold_3", &assets.touch_hold[3]),
        ("touch_effect", &assets.touch_effect),
        ("touch_effect_star_0", &assets.touch_effect_stars[0]),
        ("touch_effect_star_1", &assets.touch_effect_stars[1]),
        ("chevron", &assets.chevron),
        ("hexagon", &assets.hexagon),
        ("halo", &assets.halo),
    ];

    let mut pending = false;
    for (name, handle) in handles {
        match server.get_load_state(handle.id()) {
            Some(LoadState::Failed(err)) => {
                error!("sprite '{name}' FAILED to load: {err}");
                *done = true;
            }
            Some(LoadState::Loaded) => {}
            _ => pending = true, // NotLoaded / Loading
        }
    }

    if !pending && !*done {
        info!("all 29 note sprites loaded OK");
        *done = true;
    }
}

const RADIUS: f32 = 350.0;
const NOTE_RADIUS: f32 = 40.0;
/// Spacing between chevron arrows along a slide track. Tune visually.
const CHEVRON_SPACING: f32 = NOTE_RADIUS * 0.75;
/// Outer radius the touch-burst stars travel to/from (shared by spawner + animator).
const SPARK_STAR_RADIUS: f32 = NOTE_RADIUS * 0.6;
/// Uniform scale of each small touch-burst star.
const SPARK_STAR_SCALE: f32 = 0.25;

#[allow(unused)]
pub mod note_colors {
    use bevy::color::Color;

    // ── Core note types ────────────────────────────────────────────────

    /// Tap and hold notes (pink).
    pub const TAP: Color = Color::srgb(1.0, 0.4, 0.6);

    /// Hold notes share the same tint as tap.
    pub const HOLD: Color = TAP;

    /// Slide notes and touch-note centre (blue).
    pub const SLIDE: Color = Color::srgb(0.4, 0.8, 1.0);

    /// Touch note centre dot reuses the slide color.
    pub const TOUCH: Color = SLIDE;

    /// Paired (multi-note) overlay color (yellow).
    pub const PAIRED: Color = Color::srgb(1.0, 1.0, 0.0);

    // ── Touch-hold directional triangles ───────────────────────────────

    /// Top triangle (red).
    pub const TOUCH_HOLD_TOP: Color = Color::srgb(1.0, 0.0, 0.0);

    /// Bottom triangle (yellow — same hue as paired, but a distinct role).
    pub const TOUCH_HOLD_BOTTOM: Color = Color::srgb(1.0, 1.0, 0.0);

    /// Left triangle (green).
    pub const TOUCH_HOLD_LEFT: Color = Color::srgb(0.0, 1.0, 0.0);

    /// Right triangle (blue).
    pub const TOUCH_HOLD_RIGHT: Color = Color::srgb(0.0, 0.0, 1.0);

    /// All four directional colours in spawn order
    /// (top, bottom, left, right — matching `[Y, -Y, -X, X]`).
    pub const TOUCH_HOLD_DIRS: [Color; 4] = [
        TOUCH_HOLD_TOP,
        TOUCH_HOLD_BOTTOM,
        TOUCH_HOLD_LEFT,
        TOUCH_HOLD_RIGHT,
    ];

    // ── UI / environmental ─────────────────────────────────────────────

    /// Judgement ring and countdown arc stroke.
    pub const RING: Color = Color::WHITE;

    /// Slide track chevron tint (light cyan).
    /// Matches `bevy::color::palettes::css::LIGHT_CYAN` but kept here so
    /// every color lives in one file.
    pub const CHEVRON: Color = Color::srgb(0.878, 1.0, 1.0);

    pub const HEXAGON: Color = Color::srgb(1.0, 0.8, 0.0);

    /// Screen background.
    pub const BACKGROUND: Color = Color::srgb(0.0, 0.0, 0.0);
}
