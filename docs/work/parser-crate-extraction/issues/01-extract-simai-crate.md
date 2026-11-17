# 01 — Extract `crates/simai`

**What to build:** move `engine/src/systems/parser/` and the chart types it
produces into a new workspace crate `crates/simai`, depending on `regex` and
nothing else. `engine` depends on it with a `bevy` feature that restores the
`#[derive(Component)]` on `NoteKind`.

What moves:

| From | To |
|---|---|
| `engine/src/systems/parser/{mod,chart,note,slide,duration,error,testutil}.rs` | `crates/simai/src/` |
| `engine/src/systems/component.rs` — `ChartEvent`, `Note`, `NoteKind`, `SlideSegment`, `SlideShape`, `Duration` and the type aliases | `crates/simai/src/chart.rs` |

`TimedEvent` stays in `engine`: it is playback state, not chart data.

**Blocked by:** None.

**Status:** done (2026-11-17)

- [x] `crates/simai` added to the workspace members; `regex` is its only
      non-dev dependency
- [x] `bevy` is an *optional* dependency behind a feature of the same name;
      `NoteKind` carries `#[cfg_attr(feature = "bevy", derive(Component))]`
- [x] `engine` depends on `simai` with `features = ["bevy"]`; no change at the
      ECS query (`systems/visual/movement/mod.rs:174`) or the spawn sites
- [x] The `pub mod chart` façade in `engine/src/lib.rs` is removed — it existed
      only so `tests/corpus.rs` could reach a private module, which the crate
      root now does directly
- [x] All 63 parser tests move with the code and pass **unmodified**. A test
      that needs editing means the move changed behaviour — stop and say so
- [x] `engine/tests/corpus.rs` + `fixtures/` move to `crates/simai/tests/`,
      including the gitignore entry for `fixtures/local/`
- [x] `cargo tree -p simai` shows no `bevy`
- [x] `cargo test -p simai` and `cargo clippy -p engine --target wasm32-unknown-unknown`
      both clean

## Comments

**2026-11-17: done.** Three departures from the plan above, none changing
behaviour:

- The chart types went to `crates/simai/src/types.rs`, not `chart.rs`: the
  parser already has a `chart.rs` (file to events), which moved with it.
- The optional dependency is `bevy_ecs`, which owns the `Component` derive,
  behind a feature still named `bevy`. All of Bevy would have worked too, but
  a crate whose purpose is staying light should pull in the one piece it uses.
- `engine/src/systems/component.rs` stays as a re-export of the chart types
  plus `TimedEvent`, so no engine import path changed at all, not only the
  query and spawn sites. `engine` also no longer depends on `regex`.

The 65 inline tests and the 2 corpus tests pass with only their import
paths changed (`crate::systems::component::` to `crate::types::`); no
assertion was touched. `cargo tree -p simai` has no Bevy. Engine: 16 tests
pass, `cargo clippy -p engine --target wasm32-unknown-unknown` is clean, and
`scripts/build-wasm.sh` builds. Until ticket 03 the CI `engine` job no
longer runs the parser tests; the branch merges with 03 included.
