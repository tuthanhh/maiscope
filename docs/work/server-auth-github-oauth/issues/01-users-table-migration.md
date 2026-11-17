# 01 — `users` table migration + new dependencies

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit) after the
> original ticket file was lost during a botched directory move. Content is
> faithful to that plan's Task 1; wording is not byte-identical to the original.

**What to build:** The `users` table backing contract §4, plus the four crates the
auth tier needs. `user_sessions.refresh_token_hash` stores a SHA-256 hash of a refresh
token, never the token itself — a database leak must not hand out working
credentials.

**Blocked by:** None — first ticket of this feature.

**Status:** done

- [x] `apps/server/migrations/20261117120000_users.up.sql` creates `users`:
      `id UUID PK DEFAULT uuidv7()`, `github_id BIGINT UNIQUE NOT NULL`, `login TEXT`,
      `avatar_url TEXT`, `role TEXT NOT NULL DEFAULT 'user'`, `deleted_at TIMESTAMPTZ`,
      `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`, `updated_at` likewise
- [x] Matching `.down.sql` drops the table
- [x] `role` constrained to `user | moderator | admin`
- [x] `login` and `avatar_url` are nullable — a deleted account keeps its row
      with both nulled (see below); `github_id` is **not** nullable and is never
      cleared
- [x] No email column. The project stores no email address, and adding one later
      is a decision about privacy obligations, not a schema convenience
- [x] `apps/server/Cargo.toml` gains `oauth2`, `jsonwebtoken`, `rand` (`sha2` is
      already a dependency)
- [x] `apps/server/.env.example` gains `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`,
      `JWT_SECRET`, and the OAuth callback URL (`GITHUB_REDIRECT_URL`), each
      with a comment
- [x] `sqlx migrate run` applies cleanly against local dev Postgres
- [x] `cargo build -p server` succeeds with the new dependencies
- [x] `docs/reference/schema.md` describes `users` as built

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


**2026-11-17 — built on the `v2` branch; three departures from the plan.**

- **`users.id` is a UUID v7, not a bigserial.** Production-v2 rule 4 was
  written after this ticket and named only the song and community tables.
  Put as a choice: the id is the JWT `sub` and the attribution on every
  upload, so a sequence would publish the account count and signup order, and
  an `i64` user id would type-check wherever any other `i64` id does. Rule 4
  now names `users.id`.
- **The migration is `20261117120000_users`**, not `20260907090000`: the
  planned timestamp sorts before the `20261117…` migrations already merged,
  and a migration must sort after everything it builds on.
- **`jsonwebtoken` is held at 10**, with `rust_crypto` and no `use_pem`.
  12's `rust_crypto` backend needs RustCrypto's `rsa 0.10` / `digest 0.11`
  generation, which does not resolve against this lockfile. The backend
  feature is all-or-nothing, so RSA and EC crates compile in unused; `rsa 0.9`
  carries RUSTSEC-2023-0071, which an HS256-only server never reaches.
  `oauth2` is built without default features, dropping its bundled reqwest
  0.12: ticket 03 hands it the server's own reqwest 0.13.

**2026-11-17 — branch review.** Applied: `refresh_token_hash` is `UNIQUE`
(refresh looks a user up by it alone, so it needs the index and may match one
row at most); `users_refresh_token_has_expiry` keeps hash and expiry paired;
`users_tombstone_shape` requires a `login` on an active row and an empty,
`user`-role row on a tombstone. Decided by the maintainer: **deletion resets
`role` to `user`**, so reactivation restores the account but not its
privileges, which an admin re-grants by hand. `updated_at` stays
trigger-free; tickets 03, 04 and 06 set it in their `UPDATE`s.

**2026-11-17 — refresh tokens move to `user_sessions`** (maintainer decision,
before ticket 03). One `refresh_token_hash` on `users` meant one live
session: every login replaced it, so signing in on a phone logged the desktop
out within a JWT lifetime. `user_sessions` holds one row per signed-in device
(`user_id`, `refresh_token_hash UNIQUE`, `expires_at NOT NULL`), cascading
from `users`. Rotation touches its own row; deletion removes all of a user's
rows. The `users_refresh_token_has_expiry` CHECK is gone, replaced by
`expires_at NOT NULL`. Same migration, edited in place: nothing had deployed
it. Read the September comments above with `users.refresh_token_*` meaning
`user_sessions.*`.
