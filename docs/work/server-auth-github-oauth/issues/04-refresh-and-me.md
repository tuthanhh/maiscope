# 04 — `POST /auth/refresh` and `GET /auth/me`

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit), Task 4,
> after the original ticket file was lost during a botched directory move.

**What to build:** The two remaining §4 endpoints. `POST /auth/refresh` trades an
opaque refresh token for a fresh JWT; `GET /auth/me` returns the current user and
is the first consumer of the `AuthUser` extractor.

**Refresh tokens rotate on every use.** The old token stops working the moment a
new one is issued, so a stolen token is usable at most once and the theft becomes
visible as an unexpected logout.

**Blocked by:** 02, 03

**Status:** todo

- [ ] `POST /auth/refresh` takes `{ "refreshToken": "<opaque>" }`
- [ ] Presented token hashed and matched against `users.refresh_token_hash`;
      no match → `401`
- [ ] On success: issue a new JWT **and** a new refresh token, replacing the stored
      hash (rotation)
- [ ] Response: `{ "token": "<jwt>", "refreshToken": "<opaque>" }`
- [ ] `GET /auth/me` uses the `AuthUser` extractor and returns
      `{ id, login, avatarUrl, role }`
- [ ] `GET /auth/me` without a token, or with an expired one, returns `401`
- [ ] Both routes registered under the existing `/api/v1` nest
- [ ] Tests: refresh rotates and invalidates the previous token; `me` returns the
      authenticated user; both reject unauthenticated calls
