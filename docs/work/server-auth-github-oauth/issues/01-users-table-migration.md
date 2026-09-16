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
      `refresh_token_expires_at TIMESTAMPTZ`, `deleted_at TIMESTAMPTZ`,
      `created_at TIMESTAMPTZ`, `updated_at TIMESTAMPTZ`
- [ ] Matching `.down.sql` drops the table
- [ ] `role` constrained to `user | moderator | admin`
- [ ] `login` and `avatar_url` are nullable — a deleted account keeps its row
      with both nulled (see below); `github_id` is **not** nullable and is never
      cleared
- [ ] No email column. The project stores no email address, and adding one later
      is a decision about privacy obligations, not a schema convenience
- [ ] `apps/server/Cargo.toml` gains `oauth2`, `jsonwebtoken`, `rand` (`sha2` is
      already a dependency)
- [ ] `apps/server/.env.example` gains `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`,
      `JWT_SECRET`, and the OAuth callback URL, each with a comment
- [ ] `sqlx migrate run` applies cleanly against local dev Postgres
- [ ] `cargo build` succeeds with the new dependencies

## Comments

**2026-09-16 — deletion is a tombstone, and what that costs.**

Community uploads carry `uploader_user_id NOT NULL`, so "delete my account" has
to answer what happens to charts other people are playing. Three options were
put:

1. Hard delete with cascade — uploads die with the account. Also an undo button
   for abuse: upload junk, get reported, delete the account, take the library
   down with you.
2. Refuse deletion while uploads exist — a user with one upload can never leave.
3. Tombstone — chosen.

A deletion nulls `login` and `avatar_url`, sets `deleted_at`, and revokes
`refresh_token_hash`. The row stays, so uploads remain attributed and a
moderator still has a target. `github_id` is **retained**: without it a
re-login mints a second account and the tombstone becomes unreachable. That
makes this identity erasure, not erasure — describe it that way to users rather
than calling it deletion.

Re-authenticating with the same GitHub account **reactivates** the tombstone.

> When a ban eventually lands (deferred — see `production-v2`'s *Accepted, not
> solved*), reactivation must clear `deleted_at` and **not** `banned_at`.
> Otherwise this design hands every banned user an escape hatch: delete,
> re-authenticate, arrive clean.

`refresh_token_expires_at` exists because rotation alone bounds nothing. A
stolen token that is never used is never rotated, so without an expiry it is a
credential with an infinite lifetime — see issue 04 for the 30-day value and why
the token cannot live in a cookie.

