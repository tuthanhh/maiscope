# 12 — Hermetic multi-stage `Dockerfile` for the server

**What to build:** A multi-stage image that builds `apps/server` from the Cargo
workspace with no network services required, and ships a small runtime image.
Must produce **two** binaries: the server and the migrator (issue 13).

Notes specific to this repo:

- The workspace includes `engine` (Bevy) and `apps/host/src-tauri`. Build only
  the server package — `cargo build -p server` — or the image will drag in the
  entire Bevy dependency tree.
- `SQLX_OFFLINE=true` and the committed `.sqlx/` make the build hermetic (issue 11).
- Dependency-caching layer before the source copy, or every build recompiles the
  whole tree.

**Blocked by:** 11

**Status:** todo

- [ ] Multi-stage `Dockerfile` at repo root (workspace context) or `apps/server/`
- [ ] Builder stage pins the Rust version to match `edition = "2024"` needs
- [ ] Dependency layer cached separately from source
- [ ] `cargo build -p server --release` only — `engine`/`src-tauri` excluded
- [ ] `SQLX_OFFLINE=true` in the builder
- [ ] Slim runtime base, non-root user, only the two binaries + CA certificates
- [ ] `.dockerignore` excludes `target/`, `songs/`, `node_modules/`, `apps/host/`
- [ ] Final image size recorded in this ticket
- [ ] Verified: image builds on a machine with **no** Postgres reachable
