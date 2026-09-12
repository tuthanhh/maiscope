# 02 — Hermetic multi-stage `Dockerfile` for the server

**What to build:** A multi-stage image that builds `apps/server` from the Cargo
workspace with no network services required, and ships a small runtime image.
Must produce **two** binaries: the server and the migrator (issue 03).

Notes specific to this repo:

- The workspace includes `engine` (Bevy) and `apps/host/src-tauri`. Build only
  the server package — `cargo build -p server` — or the image will drag in the
  entire Bevy dependency tree.
- `SQLX_OFFLINE=true` and the committed `.sqlx/` make the build hermetic (issue 01).
- Dependency-caching layer before the source copy, or every build recompiles the
  whole tree.

**Blocked by:** 01

**Status:** done

- [x] Multi-stage `Dockerfile` at `apps/server/Dockerfile`, built with the repo
      root as context (the ticket allowed either location)
- [x] Builder stage pins the Rust version to match `edition = "2024"` needs —
      `rust:1.97-slim-trixie` (edition 2024 needs ≥ 1.85)
- [x] Dependency layer cached separately from source — `cargo-chef`, three stages
- [x] `cargo build -p server --release --bin server` only
- [x] `SQLX_OFFLINE=true` in the builder
- [x] Slim runtime base (`debian:trixie-slim`), non-root user (uid 10001),
      CA certificates. **One** binary for now — the migrator arrives with issue 03
- [x] `.dockerignore` — also excludes `engine/assets/` (89MB) and any `.env`
- [x] Final image size recorded in this ticket — **150MB**
- [x] Verified: image builds on a machine with **no** Postgres reachable
- [x] Verified: container serves — `docker run` + `GET /api/v1/healthcheck`

## Comments

**Prerequisite found and fixed first:** `sqlx` was compiled with no TLS feature,
so neither `sqlx-core` nor `sqlx-postgres` resolved a TLS crate. Neon requires TLS
on every connection; a plaintext loopback to the dev container hid it completely.
Fixed in a separate commit with `tls-rustls-ring-webpki` (bundled Mozilla roots, so
certificate trust does not depend on the runtime image). Note the verification
command looks at **`sqlx-core`**, not `sqlx-postgres` — that is where the TLS layer
resolves.

**`src-tauri` is gone, which removed a trap.** The ticket's `.dockerignore` line
"excludes `apps/host/`" would have broken the build while `apps/host/src-tauri` was
still a workspace member: `cargo metadata` parses every member manifest before
compiling anything, even for `-p server`, so excluding it fails every cargo command
with a manifest-not-found error. Moot now — the crate was deleted and dropped from
`members` (ADR-0003), so `apps/host/` can be excluded wholesale.

**Base image: `debian:trixie-slim`**, over distroless (~25MB) and alpine/musl.
Rejected for now because issues 03 and 05 exercise an untested deploy pipeline, and
`fly ssh console` landing in a working shell is worth the ~80MB. Revisit with
evidence once deploys are boring. Builder and runtime must stay on the same Debian
release — a mismatch surfaces as `version 'GLIBC_2.xx' not found` at container
start, which reads like a corrupt binary.

**Dependency caching: `cargo-chef`** over the hand-rolled dummy-`src` trick. The
manual approach needs stub sources per member plus an explicit `touch` of the real
sources after the copy, or cargo reuses the stub build — a silent failure mode.

**Context reduced from 301MB to 864KB / 115 files.** The offenders were
`apps/host/` (410MB), `songs/` (146MB) and `engine/assets/` (89MB). `engine/src`
and `engine/Cargo.toml` are deliberately kept: `engine` is still a workspace member,
so cargo parses its manifest and infers its `[lib]` target even though `-p server`
never compiles it.

**Layout rule: root describes the system, `apps/<x>/` describes how one app is
built.** The Dockerfile started at the repo root and the dev-Postgres compose file
under `apps/server/`, which put two container files at two levels for no reason.
Resolved by moving both: `apps/server/Dockerfile` (one app's build) and a root
`docker-compose.yml` (a database shared by the server, `bin/ingest` and the test
suite — it was never a child of `apps/server`).

Two files stay at the root against that rule, both forced by tooling:
`.dockerignore`, which Docker resolves against the build *context* rather than the
Dockerfile; and `fly.toml` (issue 03), which `flyctl` expects at the root.

Rejected: putting everything under `deploy/`. It reads tidy but every command grows
a flag, and `.dockerignore` would have to stay at the root regardless — so infra
would still be split across two levels, defeating the point. Also rejected: a
matching `apps/host/Dockerfile`. The frontend ships via Cloudflare Pages
(`web-delivery` 03), and containerising it would mean paying Fly egress on a 40MB+
wasm download — the exact cost ADR-0003 rejected.

The build context is the repo root either way, because the Cargo workspace spans
it:

```sh
docker build -f apps/server/Dockerfile -t maiscope-server:dev .
```

**Verification without Docker.** Docker was unavailable (daemon inactive, user not
in the `docker` group), so the builder stage was reproduced by hand: the surviving
context was copied to a clean directory and built with
`env -u DATABASE_URL -u SQLX_OFFLINE cargo build --release --locked -p server --bin server`.
It succeeded in 1m15s, proving the manifest set is complete, `.sqlx/` covers the
build, and no database is touched. The resulting binary is **11MB** and links only
`libgcc_s`, `libm`, `libc` — no `libssl`, confirming the pure-rustls tree. Expect a
final image around 95MB. This does **not** substitute for a real `docker build`;
layer caching, `cargo-chef` behaviour and the runtime stage are all still untested.

**Closed on user-run verification.** `docker build` and the `docker run` +
`/api/v1/healthcheck` smoke check were both run by the repo owner; Docker was never
reachable from the agent session that wrote this ticket, so those two results are
reported, not independently reproduced here.

**Actual image: 150MB**, against the ~95MB predicted from an 11MB binary on
`debian:trixie-slim`. Not investigated — 150MB is acceptable for a scale-to-zero
service and the gap is not on any critical path. If it is ever worth chasing, the
cheap first check is `docker history maiscope-server:dev`, which attributes the
bytes per layer; the likely candidates are the base image being larger than assumed
and the release binary carrying debug symbols (no `strip` is configured).
