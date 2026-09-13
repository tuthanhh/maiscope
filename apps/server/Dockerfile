# syntax=docker/dockerfile:1

# Build from the REPO ROOT, not from this directory — the Cargo workspace spans
# the repo, so the context must be the root even though this file lives here:
#
#   docker build -f apps/server/Dockerfile -t maiscope-server:dev .
#
# `.dockerignore` stays at the root for the same reason: it is resolved against
# the context, not against this file.
#
# Builds only the `server` package out of the Cargo workspace. `engine` pulls in
# the whole Bevy tree, so `--workspace` here would multiply the build by an order
# of magnitude for code the image never runs.
#
# The build is hermetic: no database is reachable during `docker build`, and the
# 64 compile-time `sqlx::query!` macros type-check against the committed `.sqlx/`
# cache instead (ticket 01).
#
# RUST_VERSION and DEBIAN_RELEASE must name the same Debian release. Builder and
# runtime share a glibc; mixing releases produces `version 'GLIBC_2.xx' not found`
# at container start, which reads like a corrupt binary rather than a base-image
# mismatch.
ARG RUST_VERSION=1.97
ARG DEBIAN_RELEASE=trixie

# --- Base with cargo-chef -----------------------------------------------------
# cargo-chef separates "compile the dependency graph" from "compile our code", so
# editing a handler does not rebuild ~400 crates. The workspace has several bin
# targets, which the hand-rolled dummy-src equivalent handles badly.
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS chef
WORKDIR /build
RUN cargo install cargo-chef --locked

# --- Plan ---------------------------------------------------------------------
# Produces a recipe describing only the dependency graph. Changing our own source
# does not change the recipe, which is what keeps the cook layer cached.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Build --------------------------------------------------------------------
FROM chef AS builder

# `.cargo/config.toml` in the source tree already sets this for the workspace;
# stated again here so the image does not silently depend on that file surviving.
ENV SQLX_OFFLINE=true

COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p server

COPY . .
# `--locked` fails the build rather than silently resolving newer patch versions,
# so the image stays a function of the commit.
#
# Two binaries only. `sync_catalog`, `seed_chart` and `seed_songs` are operator
# tools and stay out of the serving image.
RUN cargo build --release --locked -p server --bin server --bin migrate

# --- Runtime ------------------------------------------------------------------
FROM debian:${DEBIAN_RELEASE}-slim AS runtime

# sqlx bundles its own roots via `tls-rustls-ring-webpki`, but reqwest resolves
# `rustls-platform-verifier`, which reads the OS trust store on Linux. Without
# this, outbound HTTPS from `bin/sync_catalog` and the sync route fails to verify.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --user-group --no-create-home --uid 10001 maiscope

# Commit the image was built from, surfaced in the startup log so a running
# machine can be tied back to a commit. Passed by the deploy workflow as
# `--build-arg GIT_SHA=...`; defaults to "unknown" for local builds. Declared
# late so changing it invalidates only these last layers, not the Rust build.
ARG GIT_SHA=unknown
ENV GIT_SHA=${GIT_SHA}

# The migrator runs as fly.toml's release_command, before the new machine takes
# traffic. sqlx::migrate!() embedded the SQL at compile time, so no migrations/
# directory is needed here.
COPY --from=builder /build/target/release/server /app/server
COPY --from=builder /build/target/release/migrate /app/migrate

USER maiscope
WORKDIR /app

# Documentation only. `config.rs` reads PORT (default 3000) and `main.rs` binds
# 0.0.0.0, so Fly's injected PORT is honoured without changing anything here.
EXPOSE 3000

# CMD, deliberately not ENTRYPOINT. Fly implements `release_command` by setting
# the machine's *cmd*, and under Docker semantics cmd becomes arguments to the
# entrypoint. With `ENTRYPOINT ["/app/server"]` the release machine ran
# `/app/server /app/migrate` — the server ignored the stray argument, bound :3000
# and served forever, so the machine never exited, the release command never
# finished, and every deploy aborted. CMD is replaced outright, so
# release_command actually runs the migrator.
CMD ["/app/server"]
