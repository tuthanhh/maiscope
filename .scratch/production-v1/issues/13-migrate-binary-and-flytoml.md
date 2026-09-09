# 13 — `bin/migrate` + `fly.toml` with `release_command`

**What to build:** Migrations run as a Fly **release command** — a step that runs
before the new machine takes traffic — rather than at app startup.

Why not at startup: a bad migration in a release command *fails the deploy*, Fly
aborts, and the old machine keeps serving. At startup the new machine boots,
panics, restarts, panics — a crash loop instead of a clean rollback. It also
keeps schema work off the cold-start latency path, which matters on scale-to-zero.

Use a tiny `bin/migrate.rs` wrapping `sqlx::migrate!()` rather than installing
`sqlx-cli` into the runtime image.

**Blocked by:** 12

**Status:** todo

- [ ] `apps/server/src/bin/migrate.rs` — connect, `sqlx::migrate!().run()`, log
      applied versions, exit non-zero on failure
- [ ] Migrator binary included in the runtime image (issue 12)
- [ ] `fly.toml`: app name, region, `release_command = "/app/migrate"`
- [ ] `auto_stop_machines` / `auto_start_machines` on, `min_machines_running = 0`
- [ ] Memory sized small (256MB) and recorded; raise only with evidence
- [ ] `[[services]]` health check pointed at `/api/v1/healthcheck`
- [ ] `DATABASE_URL` set via `fly secrets`, never in `fly.toml`
- [ ] Verified: a deliberately broken migration aborts the deploy and the previous
      version keeps serving
- [ ] Documented rule: destructive migrations are **not** deployed this way
      (expand-and-contract — see spec.md)
