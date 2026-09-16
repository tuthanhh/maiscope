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

**Status:** done

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
- [x] **Fly app created and secret set**, plus Shared IPv4 + Dedicated IPv6
      assigned (a UI deploy with a prebuilt image does not allocate them, unlike
      a first `fly deploy`)
- [x] **Deployed and serving** — `GET https://maiscope-api.fly.dev/api/v1/healthcheck`
      returns 200 `{"status":"good"}` in ~160ms, schema at `20260907110000`
- [x] Verified: a deliberately broken migration aborts the deploy and the previous
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

**Verified against real Neon**, not just local Postgres (2026-09-13). Against an
empty `neondb` in `ap-southeast-1`, over
`?sslmode=require&channel_binding=require`: `sqlx migrate info` connected and
listed all 5 as pending, `cargo run --bin migrate` applied all 5 and exited 0, and
a second run reported `schema already current`. That also confirms the
`tls-rustls-ring-webpki` change was both necessary and sufficient — the same
connection would have failed outright before it.

**`ENTRYPOINT` broke `release_command` — four failed deploys.** Fly implements
`release_command` by setting the machine's **cmd**, and Docker appends cmd to the
entrypoint as arguments. With `ENTRYPOINT ["/app/server"]` the release machine ran
`/app/server /app/migrate`: the server ignored the extra argument, bound `:3000`
and served forever, so the machine never exited and flyctl aborted every deploy.

The symptoms were misleading. The first attempt exited 1 (that was the *server*
failing on an unset `DATABASE_URL`, not the migrator); later attempts hung; and the
release machine emitted a `listening 0.0.0.0:3000 … schema_version` line that looked
exactly like a healthy app boot. `bin/migrate` never ran on Fly at all.

Fixed by using `CMD ["/app/server"]` in `apps/server/Dockerfile`. **Any image whose
release command must be overridable has to use CMD, not ENTRYPOINT.**

**The abort path was observed four times, accidentally.** Every failed deploy left
nothing serving and created no app machine — Fly aborted before any machine took
traffic, which is the property the release command exists for. It is not the
ticket's test though: the failures were the wrong binary running, not a bad
migration, and there was no previous version to keep serving. That test is only
meaningful now that a good version is live.

**Broken-migration abort verified for real (2026-09-16).** A throwaway branch
(`scratch/broken-migration-test`, never merged, deleted after) added a
migration pair with deliberately invalid SQL
(`99999999999999_broken_test.up.sql`: `THIS IS NOT VALID SQL;`), deployed
directly with `flyctl deploy --remote-only` against the live app (v9 running):

```
migrate: applying (takes the migration advisory lock)
migrate: failed: while executing migration 99999999999999: error returned
  from database: syntax error at or near "THIS" at line 1236
Error: release command failed - aborting deployment. machine d897352c155178
  exited with non-zero status of 1
```

The release-command probe machine (`d897352c155178`) exited 1 and never took
traffic; the serving machine (`080762df263448`) never restarted — confirmed by
`git_sha: e9d028f5...` unchanged in its logs and `/api/v1/healthcheck`
returning 200 throughout. Previous version kept serving, exactly as designed.

**Caught mid-test:** the scratch branch inherited an unrelated uncommitted
change on `engine/Cargo.toml` (a package rename, unrelated to this ticket)
because `git checkout -b` carries uncommitted changes forward and
`flyctl deploy` builds from the local filesystem, not git HEAD. That renamed
package without a matching `Cargo.lock` update broke `cargo build --locked`
on the first attempt — a real build failure, but the wrong one, and it never
reached `release_command` at all. Stashed the unrelated change, reran, got the
migration failure above, then restored the stash untouched. Worth remembering:
**`flyctl deploy`'s build context is "whatever's on disk," including anything
uncommitted** — an ad hoc deploy from a branch with unrelated WIP sitting in
the working tree is not the same test as a clean checkout.

**Now fully verified.** Neon: live in `ap-southeast-1`, schema applied. Fly:
`maiscope-api` app created, `DATABASE_URL` set, and the abort path proven
against the real app rather than just reasoned about from the `ENTRYPOINT`
incident above.
