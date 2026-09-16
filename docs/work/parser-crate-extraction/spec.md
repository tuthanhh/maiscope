# Spec — Parser crate extraction

**Status:** planned
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Move the simai parser out of `engine` into `crates/simai`, a crate that does not
depend on Bevy. Decided in
[ADR-0014](../../adr/0014-simai-parser-as-its-own-crate.md).

## Problem

The parser lives inside a Bevy crate for historical reasons only. Two things now
make that cost real.

**CI pays Bevy to run parser tests.** `test-foundation` 01 added 63 tests to
`engine`, so `ci.yml` grew an `engine` job that installs four system dev
packages and compiles ~300 crates — to test functions whose only dependency is
`regex`. The same workflow goes out of its way to avoid that cost everywhere
else: the `rust` job is scoped `-p server -p shared` for exactly this reason.

That job failed on its first run for `wayland-client` not found: `wayland-sys`
resolves it through `pkg-config` at *build* time because `engine/Cargo.toml`
enables Bevy's `wayland` feature, and the apt list was short two packages. It
passed locally only because the dev machine already had them. Every one of those
four packages is needed to link a test binary that never opens a window.

**The server is about to need the parser.**
[ADR-0013](../../adr/0013-community-charts-beside-the-catalog.md) validates
uploaded chart text at submission. Depending on `engine` from `apps/server` would
put Bevy, ALSA, X11 and Wayland in the server's Docker image.

The coupling is one line: `use bevy::prelude::Component` in
`engine/src/systems/component.rs`, for a single derive on `NoteKind`.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [Extract `crates/simai`](issues/01-extract-simai-crate.md) | The move itself, behind a `bevy` feature |
| 02 | [`parse_maidata` into the library](issues/02-parse-maidata-into-library.md) | Out of `bin/seed_songs.rs` |
| 03 | [Server depends on `simai`; CI follows](issues/03-server-dependency-and-ci.md) | Blocked by 01 |

## Decisions that constrain this work

- **Feature flag, not a newtype.** `engine` keeps using `NoteKind` as a
  component with no churn at the query or spawn sites; `simai/bevy` adds the
  derive. The rejected newtype alternative is recorded in ADR-0014.
- **Nobody adds a `--workspace` build.** Cargo unifies features across crates
  built together, so a whole-workspace build would enable `simai/bevy` for the
  server's graph. The repo already avoids this for the sqlx cache and the `rust`
  CI job; this work does not change that, it depends on it.
- **Behaviour does not change.** This is a move. The 63 parser tests move with
  the code and must pass unmodified — any test that needs editing to survive the
  move is a signal the move changed something it should not have.

## Out of scope

Splitting `engine`'s other large modules. `systems/visual/spawning.rs` (640) and
`movement/mod.rs` (465) are candidates on size alone, but nothing outside the
engine needs them, so they stay where they are.

## Done when

`cargo test -p simai` runs the parser suite with no Bevy in the dependency graph,
`apps/server` can call `parse_chart` without pulling Bevy, and the `engine` CI
job no longer installs ALSA or udev headers to run parser tests.
