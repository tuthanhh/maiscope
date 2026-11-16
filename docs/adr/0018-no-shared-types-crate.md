# ADR-0018 — No shared-types crate

**Status:** accepted
**Date:** 2026-11-16

## Context

The workspace was scaffolded with a `shared/` crate meant to hold types used by
more than one tier — the server's wire shapes, mirrored for the frontend. It
never got past the scaffold: one `add(left, right)` function and its test.
Nothing imported it. CI compiled and tested it on every server PR
(`-p server -p shared`), and three documents listed it as a tier.

The reason it stayed empty is a decision made elsewhere.
[`test-foundation`](../work/test-foundation/spec.md) chose a `/catalog`
snapshot test over generated types: the server deliberately emits raw fields
only, and `preprocessData` adds `songNo`, `imageUrl`, `sheetExpr`,
`notePercents` and `$canonicalSheet` on the client, so a shared type would
describe neither what the server sends nor what the client uses. In a
read-only API the failure worth catching is a dropped or renamed field, and
the snapshot catches that — and also asserts the derived fields are *absent*,
which a type cannot.

With that decided, `shared` had no planned contents. It was a placeholder for
an approach the repo had rejected.

## Decision

**Delete `shared/`.** The workspace is `apps/server` and `engine`. CI's `rust`
job narrows to `-p server`. Cross-tier agreement is enforced by the contract
snapshot (`apps/server/tests/snapshots/catalog.json`) and by the frontend's
hand-written `types/{Data,Song,Sheet}.ts`, which
[`api-contract.md`](../reference/api-contract.md) §1.1 names as the shape the
server must match.

## Rejected alternatives

**Keep it until the v2 API rewrite.** [`api-rewrite`](../work/api-rewrite/spec.md)
is a from-scratch API with a redesigned frontend, and it is plausible v2 wants
generated types. Rejected because a stub is not a head start: whatever v2
decides, it starts by writing the types, and an empty crate named `shared`
only pre-decides the name. If v2 wants shared types it adds a crate then, with
the contents that justify it.

**Keep it because ADR-0014 mentions it.** [ADR-0014](0014-simai-parser-as-its-own-crate.md)
used `shared/`'s existence to settle where non-app crates live (root versus
`crates/`), choosing `crates/simai`. That reasoning does not need `shared` to
keep existing — the choice is recorded and `simai` simply becomes the first
crate under `crates/`. ADR-0014 carries a note to that effect.

**Wire it in now with the server's wire types.** Would make the crate real,
but reintroduces exactly what `test-foundation` rejected: a type that claims
to be the contract while the client reshapes everything it receives.

## Consequences

Three documents and one CI job stop describing a tier that does not exist.
`cargo build --workspace` no longer touches it; `Cargo.lock` drops the entry.

If a future change needs a Rust type on both sides of the wire — the v2 API
with a Rust-generated TypeScript client, say — that is the moment to add a
crate, under `crates/` per ADR-0014, named for what it holds.
