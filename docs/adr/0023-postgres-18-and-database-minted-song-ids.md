# ADR-0023 — Postgres 18 everywhere; song ids are minted by the database

**Status:** accepted
**Date:** 2026-11-17

## Context

`song-public-id` makes `songs.id` a UUID v7. Something has to mint those
ids: once for the 1879 existing songs inside a migration, and again for
every song the sync inserts later.

Production runs Postgres 18 on Neon. Dev (`docker-compose.yml`) and CI ran
Postgres 17. Postgres 18 ships `uuidv7()`; 17 does not. The ticket had
planned the `uuid` crate with its `v7` feature.

## Decision

**The column default mints: `songs.id UUID DEFAULT uuidv7()`.** The
migration's backfill calls the same function. No Rust code mints a song id;
the sync never names `id`, so a new song is minted one and an existing song
keeps its own.

**Postgres 18 is the floor for every environment.** Dev and CI move from 17
to 18, matching Neon. Migrations may use 18-only features.

The `uuid` crate is not added. The server reads and binds ids through
sqlx's re-exported `Uuid`; the crate arrives with the first code that mints
or serializes one in Rust.

## Rejected alternatives

**Mint in Rust with the `uuid` crate.** Works on any Postgres version.
Against: a SQL migration cannot call Rust, so the backfill would split into
a column-adding migration, a code step, and a key-swapping migration,
instead of one transaction. And the sync would have to mint for new rows,
which a default does for free.

**A hand-written `uuid_v7()` SQL function on Postgres 17.** No version
change. Against: about ten lines of bit layout the project would own and
test, run in production beside an equivalent built-in.

**Stay on 17 locally, 18 in production.** No dev churn. Against: the gap
already existed unnoticed, and a migration that works only in production is
exactly what CI is there to catch.

## Consequences

**Positive.** One transaction mints and swaps the key. New songs need no
code. Dev, CI and production run the same major version.

**Negative.** Every local database had to be rebuilt: the 18 image stores
its data under a different path, so compose mounts a new volume and the old
one sits unused. A future move off Neon needs a host with Postgres 18 or
later.

**Neutral.** The mint's timestamp is database time, not application time.
