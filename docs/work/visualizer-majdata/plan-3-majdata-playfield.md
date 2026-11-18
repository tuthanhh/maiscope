# Visualizer piece 3 — MajdataPlay playfield: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The playfield is drawn with MajdataPlay's art, and slide arrows are placed from MajdataPlay's prefab layouts instead of computed geometry; maiscope becomes GPL-3.0 in the same change as the first MajdataPlay file.

**Architecture:** A stdlib-only Python script reads a local MajdataPlay checkout at a pinned commit. It copies the default skin and effect sprites into `engine/assets/majdata/`. It also writes two text files: `slides.txt`, every slide prefab's arrows in child order with position and angle, and `sprites.txt`, each copied PNG's pixel size. The engine embeds both files with `include_str!` and ports MajdataPlay's shape rules: which prefab draws a segment, whether it is mirrored, and how it is turned for its start key. It spawns arrows from that data, and the trace star walks through the arrows, as MajdataPlay's star does. Note sprites switch to the new files, sized from `sprites.txt` on MajdataPlay's scale. Motion stays as it is today; only the art and the slide layouts come from MajdataPlay.

**Tech Stack:** Python 3 stdlib (extraction), Rust (Bevy 0.18), Vue/Vite (unchanged).

**Spec:** [`docs/work/visualizer-majdata/spec.md`](spec.md), piece 3.

## Global Constraints

- Licence: maiscope becomes GPL-3.0 **in the same commit as the first MajdataPlay file**. In that commit:
  - `LICENSE` is replaced by the GPL-3.0 text.
  - `LICENSE-THIRD-PARTY` gains a MajdataPlay entry.
  - `package.json` and the `Cargo.toml`s are updated.
  - An ADR records the switch.
  - `LICENSE-THIRD-PARTY` keeps its SEGA notice: the skin is likely derived from SEGA's art.
- MajdataPlay pin: `158c78a1bfd2793d9adba33d476c3b37d64599fc` (TeamMajdata/MajdataPlay, 2026-11-18). The script refuses any other commit.
- No port of MajdataPlay's code beyond the shape-selection rules: our engine, clock, audio and tests stay. Note approach, arrow appear/clear, star spin and hit-effect timing stay as the engine does them now (spec).
- No gameplay overlay: no judgement text, SlideOK, combo or score art is copied (spec, Non-goals).
- Guide sounds stay ours (spec).
- Performance: no worse than today's frame rate on the maintainer's test phone with `?debug` on a dense chart (spec). If it is worse, mobile gets smaller textures first.
- Engine boundary, wasm-bindgen pin `=0.2.122`, never `--workspace`: unchanged from plans 1–2.
- `cargo fmt --check`, `cargo clippy -p engine --target wasm32-unknown-unknown -- -D warnings`, `cargo test -p engine --lib`, `pnpm test`, `pnpm build` clean before each commit. One commit per task on `v2`, ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Decisions this plan makes (the maintainer may overrule)

1. **Licence identifier: `GPL-3.0-or-later`** for maiscope's own code.
   - Why: it is the FSF's usual form. MajdataPlay's files stay under their own GPL-3.0 terms, and the bundle as a whole is distributed under GPL-3.0.
   - Alternative: `GPL-3.0-only`.
2. **Layout data is a plain text file** (`slides.txt`), embedded with `include_str!` and parsed once at startup.
   - Rejected: generated Rust source, which would put GPL data inside `engine/src/`.
   - Rejected: serde, which the engine does not use.
3. **Holds keep our head / body / tail structure**, cut out of MajdataPlay's single `hold.png` with `Sprite.rect`. The source image has a 58 px cap at the top and at the bottom. No image tooling is needed, and the hold motion code is unchanged.
4. **The judgement ring becomes MajdataPlay's `outline.png`.** It is part of the playfield's look, and the playfield is the one thing the spec asks to match.
5. **Scale:** MajdataPlay's ring radius is 4.8 world units and its skin is 100 px per unit, so one MajdataPlay pixel becomes `RADIUS / 480` of our pixels.
   - Sprites keep MajdataPlay's proportions to the ring: a tap is 122 px there, 89 px here (80 now).
   - That is the spec's "wider slides": the arrow is 70×94 there, 51×69 here.
6. **A segment with no MajdataPlay prefab** (MajdataPlay rejects it, e.g. `1-2`) draws no arrows, and its star travels straight to the end key. The engine logs one warning per chart.
7. **The fan (`w`) slide** uses MajdataPlay's 11 `wifi_N` bars. Its three stars go straight from the start key to the three end keys, as MajdataPlay's do.
8. **The old maimai-derived sprites are deleted once nothing loads them.** `LICENSE-THIRD-PARTY`'s SEGA notice now covers the MajdataPlay skin.

## Review Focus

1. **Every slide shape from every start key** must land on its end key. This includes the mirrored ones: `q`, `qq`, `z`, `>`/`<` from the lower half, and `V` with its turn on either side.
2. **Chained slides** (`1-4-7`, `1>4-7[8:3]`, `1-4[8:1]-7[8:1]`): the path is continuous, the star crosses the joint without a jump, and the arrows clear in order.
3. **A fan slide from every start key:** the 11 bars point from the start key to the far side, and the three stars reach the three end keys.
4. **Variants pick the right art:**
   - each (paired): `slide_each`, `tap_each`, `star_each`, `hold_each`
   - break: `*_break`
   - ex: `*_ex`
   - a star with more than one slide: `*_double`
5. **Frame rate on a dense chart on the test phone**, with `?debug`, is no worse than before this plan.

Tests for 1–4 live in Tasks 2–4 (marked). Task 4's browser and phone step covers 5.

---

### Task 1: Extraction script, MajdataPlay files, GPL-3.0

**Files:**
- Create:
  - `scripts/extract-majdata.py`
  - `scripts/test_extract_majdata.py` (with a small fixture prefab inline)
  - `engine/assets/majdata/` (output)
  - `engine/assets/majdata/SOURCE.md` (commit, licence, how to re-run)
  - `docs/adr/0027-maiscope-is-gpl-3.md`
- Modify:
  - `LICENSE` (GPL-3.0 text, copied from MajdataPlay's `LICENSE`)
  - `LICENSE-THIRD-PARTY`
  - `apps/host/public/THIRD_PARTY_NOTICES.txt`
  - `README.md` (§License)
  - `apps/host/package.json`: `"license": "GPL-3.0-or-later"` (there is no root `package.json`)
  - every `[package]` `Cargo.toml` (`apps/server`, `crates/simai`, `engine`): `license = "GPL-3.0-or-later"`
  - `scripts/build-wasm.sh`: also sync `engine/assets/majdata` to `apps/host/public/assets/majdata`

**Interfaces:**
- Produces:
  - `engine/assets/majdata/slides.txt`. Lines:
    - `# …` comment
    - `prefab <Name>` (e.g. `Star_Line_5`, `Slide_Wifi`)
    - `bar <child name> <x> <y> <angle_deg>`
    - Bars appear in the root Transform's `m_Children` order (not file order).
    - `x`, `y` come from `m_LocalPosition`. `angle_deg = degrees(2·atan2(z, w))` from `m_LocalRotation`.
    - Only children named `Slide_*` or `wifi_*` are kept. The `Just_*` / SlideOK child is dropped.
  - `engine/assets/majdata/sprites.txt`. Lines: `<path relative to majdata/> <width> <height>`, read from each PNG's IHDR.
  - Copied art, at these paths under `engine/assets/majdata/`:
    - `skin/{TapSkins,StarSkins,HoldSkins,SlideSkins,WifiSkins,TouchSkins,TouchHoldSkins}/*.png`
    - `skin/outline.png`
    - `effects/{Hex,StarWhite,Star,TouchEff,TouchEffparts_01,TouchEffparts_02}.png`, from `Assets/Sprites/Game/EffectSprites/`
- CLI: `python3 scripts/extract-majdata.py --src <MajdataPlay checkout>`. It exits non-zero, with a message, unless `git -C <src> rev-parse HEAD` equals the pin.

- [ ] **Step 1: Write the failing tests** (`python3 -m unittest scripts/test_extract_majdata.py`):
  - `test_bars_follow_child_order`: a fixture prefab with three `Slide_*` transforms, listed in the file in a different order from `m_Children`. The bars come out in `m_Children` order.
  - `test_angle_from_quaternion`: `z: 0.55557024, w: 0.8314696` → `67.5` (±1e-3).
  - `test_slideok_child_is_dropped`: a `Just_str` child is not emitted.
  - `test_png_size`: a 1×1 PNG written by the test (stdlib `zlib`) → `(1, 1)`.
  - `test_refuses_other_commit`: `--src` set to a temp git repo exits 2.
- [ ] **Step 2: Run** → FAIL (module missing).
- [ ] **Step 3: Implement.**
  - Split the prefab on `--- !u!`. Index GameObjects (`!u!1`) by id for `m_Name`, and Transforms (`!u!4`) by id.
  - The root Transform is the one with `m_Father: {fileID: 0}`.
  - Use no YAML library; the fields read are fixed.
- [ ] **Step 4: Run the script** against a sparse checkout at the pin (`Assets/Prefabs/Game/Slides`, `Assets/StreamingAssets/Skins/default`, `Assets/Sprites/Game/EffectSprites`). Commit its output.
  - Expected: `slides.txt` lists 43 prefabs. `Star_Line_5`'s bars start near `(-1.58, -3.82)`, run in child order, and every angle is 67.5.
- [ ] **Step 5: Relicence**, as in Global Constraints.
  - ADR 0027 records why (MajdataPlay's art ships in the app). It also records the two rejected options from the spec (keep MIT with a GPL folder; imitate the look with our own art) and the `-or-later` decision.
  - `README.md` §License says GPL-3.0-or-later and points at `LICENSE-THIRD-PARTY`.
- [ ] **Step 6: Run** `python3 -m unittest scripts/test_extract_majdata.py`, `./scripts/build-wasm.sh` (then check that `apps/host/public/assets/majdata/skin/TapSkins/tap.png` exists), and `pnpm build`. Expected: PASS.
- [ ] **Step 7: Commit** — `feat: MajdataPlay assets and slide layouts; maiscope is GPL-3.0 (visualizer-majdata 12)`

---

### Task 2: Slide layouts in the engine (pure)

**Files:**
- Create: `engine/src/systems/visual/majdata/{mod.rs,layouts.rs,shapes.rs}` (with tests)

**Interfaces:**
- Consumes: `slides.txt`, `sprites.txt` (Task 1).
- Produces:
  - `pub struct Bar { pub name: String, pub pos: Vec2, pub angle_deg: f32 }`
  - `pub struct SlideLayouts(HashMap<String, Vec<Bar>>)`: a `Resource`, with `pub fn load() -> Self` (parses the embedded `slides.txt`) and `pub fn get(&self, prefab: &str) -> Option<&[Bar]>`.
  - `pub fn sprite_px(path: &str) -> Vec2`: the PNG's size from `sprites.txt` × `RADIUS / 480.0`. It panics in debug on an unknown path.
  - `pub struct Placement { pub prefab: &'static str, pub mirror: bool }`
  - `pub fn prefab_for(shape: &SlideShape, start: ButtonId) -> Option<Placement>`. It ports MajdataPlay's `DetectShapeFromText` (`NoteLoader.cs`, at the pin). The relative end is `((end − start) mod 8) + 1`. `MirrorKeys` maps 1↔1, 2↔8, 3↔7, 4↔6, 5↔5. `IsUpperHalf` is keys 7, 8, 1, 2.
    - `Straight`: `line{3..7}`
    - `ClockwiseArc` (`>`): `circle{e}` from the upper half, otherwise mirrored `circle{Mirror(e)}`
    - `CounterClockwiseArc` (`<`): the reverse of `>`
    - `ShortArc` (`^`): `circle{e}` for e<5, mirrored for e>5
    - `VShape`: `v{e}`
    - `PShape` / `QShape`: `pq{e}` / mirrored `pq{Mirror(e)}`
    - `GrandPShape` / `GrandQShape`: `ppqq…` the same way
    - `Thunderbolt`: `s`, or mirrored `s` when `is_z`
    - `GrandVShape`: turn 7 → `L{e}`; turn 3 → mirrored `L{Mirror(e)}`
    - `FanShape`: `wifi`
    - Prefab names: `line N→Star_Line_N`, `circle→Star_Circle_N`, `v→Star_V_N`, `pq→Star_pq_N`, `ppqq→Star_ppqq_N`, `s→Star_S`, `L→Star_L_N`, `wifi→Slide_Wifi`.
    - `None` where MajdataPlay throws.
  - `pub fn place(bar: &Bar, start: ButtonId, mirror: bool) -> (Vec2, f32)`: our pixel position and rotation in radians.
    - Mirror: `x → -x` and `angle → 180° − angle`, then rotate by `−45°·start`.
    - Otherwise rotate by `−45°·(start−1)`.
    - Then scale by `RADIUS / 4.8`.
    - (MajdataPlay `SlideDrop.Init`: `localScale.x = -1` with `Euler(0,0,-45·StartPos)` when mirrored, `Euler(0,0,-45·(StartPos-1))` otherwise.)

- [ ] **Step 1: Write the failing tests**:
  - `every_prefab_the_rules_name_exists` (Review Focus 1): for every `SlideShape` variant and end key, from every start 1–8, `prefab_for` is either `None` or names a prefab `SlideLayouts::load()` has, with at least 3 bars.
  - `slides_land_on_their_end_key` (Review Focus 1): for every `Some` placement, the last placed bar is within `0.6·NOTE_RADIUS` of the end key's ring position, and the first is within `1.6·NOTE_RADIUS` of the start key's. Use `ButtonLayout::default().tap[k-1] * RADIUS`. For `GrandV`, the end is `end`. For fans, use the middle end.
  - `mirrors_match_majdataplay`: `1q5` → `Placement { prefab: "Star_pq_5", mirror: true }`; `5>1` (lower half) is mirrored; `1>5` is not.
  - `the_corpus_has_a_layout_for_every_slide`: every slide segment in `crates/simai/tests/fixtures/*` maps to `Some`, or is listed in the test as a known invalid chart.
  - `sprite_sizes_follow_majdataplay_scale`: `sprite_px("skin/TapSkins/tap.png") == Vec2::splat(122.0 * RADIUS / 480.0)`.
- [ ] **Step 2: Run** `cargo test -p engine --lib majdata` → FAIL.
- [ ] **Step 3: Implement.** If `slides_land_on_their_end_key` fails for a whole family, the placement formula is wrong, not the data: compare against MajdataPlay's `SlideDrop.Init` before changing anything.
- [ ] **Step 4: Run** → PASS; clippy for wasm.
- [ ] **Step 5: Commit** — `feat(engine): MajdataPlay slide layouts and shape rules (visualizer-majdata 13)`

---

### Task 3: Slides drawn from the layouts

**Files:**
- Modify: `engine/src/systems/visual/spawning.rs` (slide and fan spawning), `slide_path/trace.rs`, `shapes.rs`, `schedule.rs` (test only), `systems/visual/mod.rs` (insert `SlideLayouts` at startup)
- Delete: `slide_path/generators.rs`, and every function in `slide_path/geometry.rs` that only they use

**Interfaces:**
- Consumes: Task 2 entirely; `slide_span_secs` (piece 2).
- Produces: `pub fn build_slide_from_layout(segments: &[SlideSegment], start: ButtonId, note_bpm: f32, shared_duration: bool, layouts: &SlideLayouts, layout: &ButtonLayout) -> (SlidePath, Vec<(Vec2, f32)>)`.
  - `SlidePath` keeps its shape. `waypoints` = start key position, then every segment's placed bars in order, each segment starting at the previous segment's end key.
  - `breakpoints` = per segment `(cumulative secs, cumulative distance)` as today.
  - The `Vec` = each arrow's position and rotation, with `SlideArrow::distance_along_path` = its waypoint's distance.
  - A segment with no placement contributes a straight line to its end key and no arrows (Decision 6).

Arrows are sprites of `skin/SlideSkins/slide.png` (or `slide_each`, `slide_break`), at `sprite_px` size, rotated by the bar angle. The fan uses the `skin/WifiSkins/wifi_N.png` bars (`wifi_each_N`, `wifi_break_N`). Its three lanes are straight from the start key to `ends.0/1/2`. The trace star is rotated by the travel angle + 18°, as MajdataPlay's (`SlideDrop.LoadSlidePath`).

- [ ] **Step 1: Write the failing tests** in `spawning.rs` / `trace.rs`:
  - `a_slide_has_one_arrow_per_prefab_bar`: `1-5[8:1]` spawns as many `SlideArrow` children as `Star_Line_5` has bars.
  - `a_chained_slide_is_continuous` (Review Focus 2): for `1-4[8:1]-7[8:1]` and `1>4-7[8:3]`, consecutive waypoints are never more than `1.5 × the largest bar spacing` apart. `breakpoints` has one entry per segment.
  - `arrows_clear_in_order` (Review Focus 2): the arrows' `distance_along_path` is non-decreasing in spawn order.
  - `a_fan_has_eleven_bars_and_three_lanes` (Review Focus 3): `1w5[8:1]` from starts 1, 4 and 7 has 11 arrows, and its lanes end within `0.6·NOTE_RADIUS` of keys 4, 5 and 6 rotated by the start.
  - `slide_span_matches_the_built_path` (schedule.rs) now builds with `build_slide_from_layout`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** Delete the geometry generators and lyon chevrons once nothing calls them. `cargo build` and the tests show what is left.
- [ ] **Step 4: Run** the full engine suite → PASS; clippy for wasm; `./scripts/build-wasm.sh`.
- [ ] **Step 5: Browser check:**
  - Look at one chart per shape family (`-`, `^`, `>`, `<`, `v`, `p`, `q`, `pp`, `qq`, `s`, `z`, `V`, `w`, a chain), each from a start in the upper half and one in the lower half.
  - Screenshot each into the scratchpad. The maintainer compares them with MajdataPlay, since there are no reference images in the repo.
  - Seek into the middle of each family.
- [ ] **Step 6: Commit** — `feat(engine): slides drawn from MajdataPlay layouts (visualizer-majdata 14)`

---

### Task 4: Note, hold, touch and effect art; the ring

**Files:**
- Modify: `engine/src/systems/visual/resources.rs` (`NoteAssets` paths), `shapes.rs` (sizes from `sprite_px`), `spawning.rs` (holds via `Sprite.rect`), `judgement_circle.rs` (ring from `skin/outline.png`), `docs/work/visualizer-majdata/spec.md` (piece 3 status)
- Delete: the old `engine/assets/sprites/` files nothing loads any more, and the `sprites` sync in `build-wasm.sh` if the folder is gone

**Interfaces:**
- Consumes: `sprite_px` (Task 2).

Mapping. Each variant uses its `_each` / `_break` / `_ex` / `_double` file where the skin has one (Review Focus 4):
- `tap` → `skin/TapSkins/tap*.png`
- `star` → `skin/StarSkins/star*.png`
- hold head / body / tail → `skin/HoldSkins/hold*.png`
  - `rect` regions: top 58 px, middle, bottom 58 px
  - **check the orientation in the browser; the cap that sits on the ring is the head**
- touch petal → `skin/TouchSkins/touch.png`, centre → `touch_point.png`, each border → `touch_border_2/3.png`, just → `touch_just.png`
- touch hold → `skin/TouchHoldSkins/touchhold_{0..3}.png` and `touchhold_border.png`
- effects → `effects/*`
- ring → `skin/outline.png`, at `sprite_px` size, centred

- [ ] **Step 1: Write the failing tests**:
  - `note_sizes_follow_majdataplay_scale`: the spawned tap, star and touch petal sprites have `custom_size == sprite_px(<file>)`.
  - `variants_pick_their_art` (Review Focus 4): a paired `1/5` tap uses `tap_each.png`; `1b` uses `tap_break.png`; `1x` uses `tap_ex.png`; a paired `1-5/5-1` slide's arrows use `slide_each.png`; a star with two slides uses `star_double.png`. Compare against the `NoteAssets` handles.
  - `a_hold_is_cut_from_one_image`: a hold's head / body / tail sprites share the `hold.png` handle, with `rect` heights 58, `h − 116`, 58.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**, then delete the sprites nothing loads.
- [ ] **Step 4: Run** the full engine suite, clippy, `./scripts/build-wasm.sh`, `pnpm build` → PASS.
- [ ] **Step 5: Browser and phone check** (Review Focus 5):
  - On a dense chart, with `?debug`, compare the frame rate before (`master`) and after, on the desktop and on the test phone.
  - Check light and dark page themes, and fullscreen.
  - If the phone is slower: halve the texture sizes for mobile first, as the spec says.
- [ ] **Step 6: Commit** — `feat(engine): MajdataPlay note, hold, touch and effect art (visualizer-majdata 15)`; it also sets the spec's piece-3 status to done.
