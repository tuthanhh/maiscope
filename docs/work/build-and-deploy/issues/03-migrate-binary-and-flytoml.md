# 03 — `bin/migrate` + `fly.toml` with `release_command`

**What to build:** Migrations run as a Fly **release command** — a step that runs
before the new machine takes traffic — rather than at app startup.

Why not at startup: a bad migration in a release command *fails the deploy*, Fly
aborts, and the old machine keeps serving. At startup the new machine boots,
panics, restarts, panics — a crash loop instead of a clean rollback. It also
keeps schema work off the cold-start latency path, which matters on scale-to-zero.

Use a tiny `bin/migrate.rs` wrapping `sqlx::migrate!()` rather than installing
`sqlx-cli` into the runtime image.

**Blocked by:** 02

**Status:** in-progress

- [x] `apps/server/src/bin/migrate.rs` — connect, `sqlx::migrate!().run()`, log
      applied versions, exit non-zero on failure
- [x] Migrator binary included in the runtime image (issue 02)
- [x] `fly.toml` **at the repo root**: `app = "maiscope-api"`,
      `primary_region = "sin"`, `release_command = "/app/migrate"`, and
      `[build] dockerfile = "apps/server/Dockerfile"`
- [x] `?sslmode=require` documented in `fly.toml`'s header for the Neon URL
- [x] `auto_stop_machines = "stop"` / `auto_start_machines = true`,
      `min_machines_running = 0`
- [x] Memory 256MB, `shared` CPU — recorded here; raise only with evidence
- [x] Health check on `/api/v1/healthcheck` (`[[http_service.checks]]`; the old
      `[[services]]` syntax is superseded)
- [x] `DATABASE_URL` kept out of `fly.toml` — verified by parsing the file and
      asserting the string does not appear
- [ ] **Fly app created and secret set** — needs a Fly account; not done from the
      agent session (see the repo rule about never touching production)
- [ ] Verified: a deliberately broken migration aborts the deploy and the previous
      version keeps serving
- [x] Documented rule: destructive migrations are **not** deployed this way
      (expand-and-contract) — stated in both `fly.toml` and `migrate.rs`

## Comments

**Migrator verified locally, all four paths:**

| Case | Result |
|---|---|
| empty database | applies all 5, logs each `version description` |
| already current | `schema already current (5 migration(s) applied previously)`, exit 0 |
| bad credentials | `migrate: failed: …`, **exit 1** |
| `DATABASE_URL` unset | `migrate: failed: DATABASE_URL is not set`, **exit 1** |

Exit 1 is the whole mechanism: Fly aborts the deploy on a non-zero release
command, so a failed migration leaves the previous version serving. Tested
against a throwaway `maiscope_migrate_probe` database, then dropped.

**Uses the runtime `sqlx::query_scalar` function, not the macro**, for the
"which versions were already applied" snapshot. That keeps `migrate.rs` out of
`.sqlx/` entirely — one less thing to regenerate, and the binary stays buildable
even if the cache is stale.

**`migrate!()` yields both directions** for reversible migrations, so the log
filters on `migration_type.is_up_migration()`. Without it every version would be
reported twice.

**CI now uses this binary instead of `sqlx-cli`** (issue 04). The migration step
is `cargo run --bin migrate`, so CI exercises the same code path as the deploy
rather than a second implementation, and no extra tool is installed on the runner.

**Still owed, and only the repo owner can do it:** `flyctl` is not installed here,
and creating the app, setting `DATABASE_URL` via `fly secrets`, and proving that a
deliberately broken migration aborts a deploy all require a real Fly account and
a real database. Those two boxes stay open.
