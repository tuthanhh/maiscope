# ADR-0025 — `simai` derives through `bevy_ecs`; the engine re-exports its types

**Status:** accepted
**Date:** 2026-11-17

## Context

[ADR-0014](0014-simai-parser-as-its-own-crate.md) moved the parser into
`crates/simai` with an optional `bevy` feature that restores
`#[derive(Component)]` on `NoteKind`, and wrote that derive against
`bevy::prelude::Component`. It also had the engine import the chart types
from `simai` directly.

Two narrower choices came up during the move (`parser-crate-extraction`
ticket 01).

## Decision

**The optional dependency is `bevy_ecs`, the crate that owns `Component`,**
behind a feature still named `bevy`. The engine's Bevy re-exports the same
`bevy_ecs`, so the derive is the trait the engine queries.

**`engine/src/systems/component.rs` re-exports the chart types** from
`simai` and keeps `TimedEvent`, which is playback state. Every engine import
path stays as it was.

## Rejected alternatives

**Optional `bevy` itself, as ADR-0014 wrote it.** One name, matching the
feature. Against: a crate whose purpose is staying light would pull all of
Bevy to use one derive, and Bevy's default features would need switching
off by hand to keep that from growing.

**Rewriting engine imports to `simai::` paths.** No pass-through module.
Against: dozens of edits across `systems/visual/` for no behaviour change,
in the one commit that most needs a reviewable diff.

## Consequences

**Positive.** `simai` with the feature on links `bevy_ecs` only. The move
touched no engine import.

**Negative.** `component.rs` is now mostly a pass-through, a layer a reader
has to see through. A Bevy upgrade must move `simai`'s `bevy_ecs` version in
step with the engine's `bevy`, or the derive implements a different trait.

**Neutral.** ADR-0014's decision stands; only its code sample is out of
date.
