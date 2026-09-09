# 01 — `users` table migration + new dependencies

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit) after the
> original ticket file was lost during a botched directory move. Content is
> faithful to that plan's Task 1; wording is not byte-identical to the original.

**What to build:** The `users` table backing contract §4, plus the four crates the
auth tier needs. `refresh_token_hash` stores a SHA-256 hash of the current refresh
token, never the token itself — a database leak must not hand out working
credentials.

**Blocked by:** None — first ticket of this feature.

**Status:** todo

- [ ] `apps/server/migrations/20260907090000_users.up.sql` creates `users`:
      `id BIGSERIAL PK`, `github_id BIGINT UNIQUE NOT NULL`, `login TEXT NOT NULL`,
      `avatar_url TEXT`, `role TEXT NOT NULL DEFAULT 'user'`, `refresh_token_hash TEXT`,
      `created_at TIMESTAMPTZ`, `updated_at TIMESTAMPTZ`
- [ ] Matching `.down.sql` drops the table
- [ ] `role` constrained to `user | moderator | admin`
- [ ] `apps/server/Cargo.toml` gains `oauth2`, `jsonwebtoken`, `rand` (`sha2` is
      already a dependency)
- [ ] `apps/server/.env.example` gains `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`,
      `JWT_SECRET`, and the OAuth callback URL, each with a comment
- [ ] `sqlx migrate run` applies cleanly against local dev Postgres
- [ ] `cargo build` succeeds with the new dependencies
