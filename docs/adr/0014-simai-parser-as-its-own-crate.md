# ADR-0014 — The simai parser is its own crate

**Status:** accepted
**Date:** 2026-09-16

## Context

The simai parser lives in `engine/src/systems/parser/`, inside a crate that
depends on Bevy. That has always been slightly wrong — parsing text has nothing
to do with an ECS — but it cost nothing until two things landed.

**The parser grew a test suite.** `test-foundation` 01 added 63 tests. CI runs
them in an `engine` job that installs `libasound2-dev` and `libudev-dev` and
compiles Bevy's ~300-crate tree, to test functions whose only dependency is
`regex`. `ci.yml` already documents the cost and the workaround for the *other*
jobs: the `rust` job is scoped `-p server -p shared` precisely so it never
compiles `engine` natively, and CLAUDE.md repeats the rule for
`cargo sqlx prepare`.

**Something else needs the parser.**
[ADR-0013](0013-community-charts-beside-the-catalog.md) validates uploaded chart
text at submission. Without extraction, `apps/server` must depend on `engine`,
dragging Bevy, ALSA, X11 and Wayland into the server's Docker image — which is
exactly what every other decision in this repo has bent to avoid.

The coupling turns out to be one line. `engine/src/systems/component.rs:1` is
`use bevy::prelude::Component`, for a single `#[derive(Component)]` on
`NoteKind`. Every module under `parser/` imports nothing but `super::`,
`crate::`, `std::` and `regex`. `NoteKind` needs the derive for exactly one ECS
query (`systems/visual/movement/mod.rs:174`) and the spawn sites; everywhere else
it is plain pattern matching that does not care.

## Decision

Extract the parser and the chart types into `crates/simai`, a crate with no Bevy
dependency. `engine` depends on it with a `bevy` feature that adds the
`Component` derive back:

```rust
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Component))]
pub enum NoteKind { … }
```

`apps/server` depends on `simai` without that feature.

`parse_maidata` moves out of `bin/seed_songs.rs` into the same crate, so the
seeder and the upload endpoint share one implementation of the `&key=` format
rather than growing two.

## Rejected alternatives

**A newtype wrapper in `engine`** — `#[derive(Component)] struct NoteKindC(simai::NoteKind)`.
Keeps Bevy out of `simai` entirely, with no feature flag and no unification
hazard. Rejected because it churns the query and spawn sites for a problem the
feature flag solves without touching them, and a wrapper that exists only to
carry a derive is a thing future readers have to decode.

**Leave the parser where it is and let the server depend on `engine`.** Zero
refactoring. Rejected outright: it puts Bevy in the server image and makes the
`rust` CI job compile the engine natively, undoing two decisions this repo
already made deliberately.

**Leave it, and reimplement validation server-side.** No dependency either way.
Rejected as the worst option — two simai parsers that must agree, where the whole
point of validating on upload is that the server accepts exactly what the engine
can render.

## Consequences

The `engine` CI job can drop its apt install and most of its cache: the parser
tests move to a crate that builds in seconds. `engine` still needs a native or
wasm job for its own code, but no longer to run parser tests.

**Cargo feature unification is the hazard.** With `resolver = "2"`, features are
still unified across crates built together, so `cargo build --workspace` would
enable `simai/bevy` for the server's copy of the graph. This is survivable
because the repo already avoids whole-workspace builds for exactly this family of
reasons — CI's `rust` job runs `-p server -p shared`, and `cargo sqlx prepare`
runs `-- -p server`. Both are documented in CLAUDE.md. Anyone adding a
`--workspace` build step needs to know this, which is the main reason it is
written down here.

`shared/` already exists as a workspace crate, so the layout question — whether
non-app crates live at the root or under `crates/` — is decided by this ADR for
the first time. `crates/simai` is the choice; `shared` stays where it is rather
than being moved for symmetry.

The public surface of `simai` becomes an interface with two consumers, so the
`pub mod chart` façade added to `engine/src/lib.rs` for the corpus test is
replaced by the crate's own root. That façade was a workaround for `systems`
being private; it stops being needed.
