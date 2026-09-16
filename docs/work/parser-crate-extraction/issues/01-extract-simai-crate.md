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

**Status:** todo

- [ ] `crates/simai` added to the workspace members; `regex` is its only
      non-dev dependency
- [ ] `bevy` is an *optional* dependency behind a feature of the same name;
      `NoteKind` carries `#[cfg_attr(feature = "bevy", derive(Component))]`
- [ ] `engine` depends on `simai` with `features = ["bevy"]`; no change at the
      ECS query (`systems/visual/movement/mod.rs:174`) or the spawn sites
- [ ] The `pub mod chart` façade in `engine/src/lib.rs` is removed — it existed
      only so `tests/corpus.rs` could reach a private module, which the crate
      root now does directly
- [ ] All 63 parser tests move with the code and pass **unmodified**. A test
      that needs editing means the move changed behaviour — stop and say so
- [ ] `engine/tests/corpus.rs` + `fixtures/` move to `crates/simai/tests/`,
      including the gitignore entry for `fixtures/local/`
- [ ] `cargo tree -p simai` shows no `bevy`
- [ ] `cargo test -p simai` and `cargo clippy -p engine --target wasm32-unknown-unknown`
      both clean
