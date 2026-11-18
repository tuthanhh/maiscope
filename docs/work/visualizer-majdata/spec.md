# Spec — Visualizer: a MajdataPlay playfield

**Status:** approved 2026-11-18 (after a builder check); next, the plan
**Milestone:** after v2.0 ships
**Parent:** none (follows [`frontend-redesign`](../frontend-redesign/spec.md))

Make the Play page a chart viewer worth using: it starts clean, pauses
dead, can go fullscreen, can be scrubbed along a timeline, and its playfield
looks like [MajdataPlay](https://github.com/TeamMajdata/MajdataPlay)'s.

## Why

The maintainer's words (2026-11-18): the player keeps the last chart when
you come back to it; slides are too thin; slide tracks are computed by
hand where the official game places them so they meet the touch points
naturally; pause lets notes already on screen keep moving; there is no
way to jump to a part of the chart. MajdataPlay is the reference for how
the playfield should look. The visualizer is a viewer, not a game.

## Goals

1. **Clean on arrival.** Opening Play shows an empty playfield unless the
   URL names a chart (`?song=&type=&difficulty=`). Leaving Play unloads the
   chart and any loaded audio, and empties the pasted text.
2. **Pause freezes everything at once** — every note, slide arrow, hold,
   touch and effect stops where it is, and resumes from there.
3. **Fullscreen as an option** — a toggle that fills the screen with the
   playfield, its transport and the timeline; Escape (or the toggle)
   leaves.
4. **A timeline** like the maintainer's reference image (described under
   piece 2): a density graph of the chart's notes, measure by measure,
   stacked by kind, with time marks every 30 s, a playhead, and a slider
   under it numbered in measures. Clicking or dragging anywhere on it
   seeks — chart, notes on screen and music together.
5. **A MajdataPlay playfield**: its note, hold, slide, touch, break and
   effect art; slide arrows at its widths; slide tracks laid out from its
   per-shape data, not computed geometry.

## Non-goals

- **No gameplay overlay.** No combo, score, judgement text or song panel
  around the ring — "the gameplay is too much for the visualization."
- **No port of MajdataPlay's code.** Its art and layout data come in; our
  Bevy engine, clock, audio and tests stay.
- **No new UI components for the controls.** The speed sliders and
  transport stay as they are, except where the timeline needs room. A
  component library is a follow-up (below).

## Anti-goals (would make this a failure even if it "works")

- Notes drifting from the music after a seek, a pause or a speed change.
- A timeline that disagrees with what plays (a burst on the graph where the
  chart is quiet).
- A playfield that looks like MajdataPlay on desktop and stutters on a
  mid-range phone (ADR-0016 ships the visualizer on mobile).

## Constraints

- **Licence: maiscope becomes GPL-3.0** (the maintainer's choice, option A
  of three). MajdataPlay is GPL-3.0; its art ships in the app. The switch
  happens in the same change as the first MajdataPlay file: `LICENSE`
  replaced, `LICENSE-THIRD-PARTY` gains a MajdataPlay entry, `package.json`
  and the `Cargo.toml`s updated, and an ADR records it. Rejected: keeping
  MIT with the assets in a GPL folder (the published bundle is GPL in
  practice anyway); imitating the look with our own art (slide layouts by
  hand).
- **The engine boundary** (CLAUDE.md): JS talks to Bevy only through the
  mailbox in `engine/src/wasm_bridge.rs`, drained once per frame. Seek,
  unload and the timeline query go through it; nothing calls Bevy
  synchronously.
- **The wasm-bindgen pin** (`=0.2.122`) and `scripts/build-wasm.sh` stay;
  never `cargo build --workspace`.
- **Guide sounds** follow the clock with a derived lead (ADR-0020); a seek
  must not fire a burst of guide sounds for the skipped notes.
- **The parser is the single source of note times** (`crates/simai`); the
  timeline is built from the same parsed chart the engine plays.

## Approaches

For the playfield, the large piece:

- **A. Extract once, render in our engine** — *chosen.* A script reads
  MajdataPlay's Unity assets and writes the sprites into
  `engine/assets/majdata/` and each slide shape's hand-placed arrow
  positions and angles into a data file the engine loads. Our renderer draws
  from that data. Keeps our engine, clock and tests; the art and layouts
  match theirs.
- **B. Port MajdataPlay's C# rendering to Rust.** Closest match, but a
  rewrite of a Unity codebase, far more work, and tied to their internals.
  Rejected.
- **C. Our own art, hand-traced layouts.** No licence change, but the
  layouts are the hard part and would be guesswork. Rejected with the
  licence choice.

## The design

Three pieces, in this order, each shippable on its own (the maintainer
agreed to the order).

### 1. Player behaviour — small

- **Cause of the pause bug** (found): every note's animation runs on its
  own `Timer`, ticked by wall time (`timer.tick(time.delta())` in
  `engine/src/systems/visual/movement/mod.rs`). Pause stops the chart clock
  (`ChartPlayback::pause`) and the music, not those timers.
- **Fix: every on-screen element is a function of chart time.** A note's
  phase and position are computed from `ChartPlayback`'s elapsed chart
  time and the note's own times, not accumulated per frame. Pause, speed
  changes and (in piece 2) seeking then follow from the one clock with no
  per-note bookkeeping. Effects that are not tied to a note (hit sparks)
  also read chart time.
- **Unload on leave.** A new mailbox command `Unload` despawns every
  note, clears the loaded chart and stops and drops the music. The page
  sends it on unmount and stops replaying a previous chart on mount (the
  `hasChart() && !hasDeepLink` branch in `pages/visualizer.vue` goes). The
  pasted text and sheet info reset with the page.
- **Fullscreen.** A toggle in the transport requests fullscreen on the
  stage element (canvas, transport, timeline). The Fullscreen API is
  missing for non-video elements on iPhone Safari; there the toggle is
  hidden.

### 2. Timeline and seeking — medium

- **`Seek(t)`** mailbox command: sets the chart clock to `t`, despawns
  everything on screen, finds the next note to spawn by binary search over
  the parsed times, re-spawns what should already be visible at `t`, and
  seeks the music (kira `seek_to`). Guide sounds for skipped notes do not
  fire (the guide-sound cursor moves with the clock).
- **`timeline()`** query, exported over wasm-bindgen: from the loaded chart,
  the chart's length, the start time of every **measure** (four beats at
  the BPM in force — simai has no time signature, maimai charts are 4/4),
  and for each measure its note counts by kind. Read on load; the graph and
  the slider are drawn from it.
- **Position readout.** The current chart time is published with the
  existing frame stats or a sibling call, polled at the display's frame
  rate while the page is open, to move the playhead.
- **UI** (`components/visualizer/`): a `VizTimeline` above the speed
  sliders, as in the maintainer's reference image (2026-11-18):
  - **The graph:** one bar per measure, its height the number of notes in
    that measure, stacked by kind — **pink: tap** (taps and breaks of tap
    kind), **blue: slide** (a slide counts by its star tap), **yellow:
    touch** (touches and touch holds), and a fourth colour (orange in the
    image) for holds. Time marks at 0:30, 1:00, … across the top, and a
    vertical playhead.
  - **The slider** under it: numbered in measures (0, 10, 20 …) with a
    tick per measure, its thumb at the playhead.
  - Clicking or dragging either seeks; keyboard: arrows step one measure,
    Page Up/Down ten, Home/End. Disabled until a chart is loaded.

### 3. MajdataPlay playfield — large

- **Extraction script** (`scripts/extract-majdata.*`, run by hand against a
  local MajdataPlay checkout at a pinned commit): copies the needed sprites
  into `engine/assets/majdata/` and writes the slide layouts — per slide
  shape and start/end button, the ordered arrow positions and rotations, as
  MajdataPlay places them — to a data file in the same folder. The pinned
  commit and the script are recorded so it can be re-run.
- **Renderer**: slide arrows are spawned from the layout data at their
  positions and angles, at MajdataPlay's sprite size (the "wider slides");
  `slide_path/`'s computed geometry is removed once every shape the parser
  produces has a layout. Note, hold, touch, break and effect sprites swap
  to MajdataPlay's art; the existing maimai-derived sprites are retired.
- **Wifi (fan) slides and touch holds** follow the same rule: their
  MajdataPlay art and layout, not ours.
- **Art and layouts, on our motion** (the builder's proposal, the
  maintainer's to correct): note approach, slide arrows appearing and
  clearing as the star passes, star spin and hit-effect timing stay as our
  engine does them. Whether to match MajdataPlay's motion too is decided
  after seeing the new art running; it would be its own piece (re-deriving
  each behaviour from their C#).
- **Where it comes from** (checked 2026-11-18): the slide shapes are data —
  Unity prefabs under `Assets/Prefabs/Game/Slides` (`Just_str`,
  `Just_curv`, `Just_wifi` and per-shape prefabs) — so loading layouts
  works; the note art is a skin under `Assets/StreamingAssets/Skins`. That
  skin is likely derived from SEGA's art, as ours is, so
  `LICENSE-THIRD-PARTY` keeps its SEGA notice for it alongside the
  GPL-3.0 entry. The guide sounds stay ours (the builder's call).

## Performance

Smooth on the maintainer's test phone means **no worse than today's frame
rate there**, measured with the `?debug` overlay on a dense chart (the
builder's proposal). If MajdataPlay's art misses it, mobile gets smaller
textures first; effects are dropped only after that, and only on phones.

## Left to the builder

- Reading the prefabs under `Assets/Prefabs/Game/Slides` and the default
  skin, and the layout data file's format.
- The exact shades of the four timeline colours, matched to the new
  sprites.
- Whether fullscreen falls back to a CSS "fill the viewport" mode on
  iPhone instead of hiding the toggle.
- Ticket boundaries within each piece.

## Follow-up, not this spec

**A component library for the app's controls.** The maintainer finds the
hand-written controls (the speed sliders among them) unpolished and would
rather adopt existing components that match the style. A separate
brainstorm: likely a headless Vue library styled with our tokens, so the
look stays ours. The visualizer's controls move to it then.

## Done when

Play opens empty; pause freezes every element; fullscreen toggles; the
timeline seeks the chart and music together and matches what plays; the
playfield's notes and slides are MajdataPlay's, with slide tracks from
their layouts; it runs smoothly on a mid-range phone; maiscope is GPL-3.0
with the ADR and third-party notices in place.
