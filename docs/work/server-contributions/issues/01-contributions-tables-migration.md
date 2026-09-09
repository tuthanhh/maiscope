# 01 — contributions, audit_log, chart_revisions tables migration

**What to build:** `contributions` table (`kind` CHECK song|sheet|chart|edit, `payload` JSONB, `status` CHECK pending|approved|rejected|merged, author/reviewer FKs to `users`), `audit_log` (append-only moderator/admin action trail), `chart_revisions` (chart content history). Also wires the `charts.contributor_user_id` FK to `users` that the earlier `charts` migration deferred. First confirm via `ls apps/server/migrations/ | grep -i chart` whether `charts` already exists (it should — this migration only adds `chart_revisions`, not `charts` itself).

**Blocked by:** `server-auth-github-oauth/issues/01-users-table-migration.md` (users table must exist for the FKs).

**Status:** ready-for-agent

- [ ] Confirmed `charts` table already exists (migration only adds `chart_revisions`, doesn't duplicate `charts`)
- [ ] `contributions` table: `kind`/`status` CHECK constraints, `author_user_id`/`reviewed_by_user_id` FKs to `users`, indexes on `status` and `author_user_id`
- [ ] `audit_log` table: `actor_user_id` FK, `contribution_id` FK, indexed on `contribution_id`
- [ ] `chart_revisions` table: `chart_id` FK (cascade delete), `contribution_id` FK
- [ ] `charts.contributor_user_id` FK to `users` added via `ALTER TABLE`
- [ ] `sqlx migrate run && sqlx migrate revert && sqlx migrate run` round-trips cleanly
