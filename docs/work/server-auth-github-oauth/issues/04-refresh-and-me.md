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

**Status:** done

- [x] `POST /auth/refresh` takes `{ "refreshToken": "<opaque>" }`
- [x] Presented token hashed and matched against `user_sessions.refresh_token_hash`;
      no match → `401`
- [x] A token past `user_sessions.expires_at` → `401`, even if the hash
      matches
- [x] A user with `deleted_at` set → `401`; reactivation happens through the
      OAuth callback (issue 03), never through refresh
- [x] On success: issue a new JWT **and** a new refresh token, replacing that
      session row's hash and setting its `expires_at` to **now + 30 days**
      (rotation). Other sessions of the same user are untouched
- [x] Rotation is one conditional `UPDATE user_sessions … WHERE
      refresh_token_hash = $old AND expires_at > now()` joined to a live
      (`deleted_at IS NULL`) user, `RETURNING …`, so of two concurrent refreshes
      with one token exactly one succeeds; it sets `updated_at = now()`
- [x] CORS admits the auth routes: `POST` and `DELETE` beside `GET`, and the
      `Authorization` and `Content-Type` request headers. Today the layer allows
      `GET` only, so every authenticated cross-origin call fails preflight
- [x] Response: `{ "token": "<jwt>", "refreshToken": "<opaque>" }`
- [x] A body that is not `{ "refreshToken": string }` → `400` in the contract's
      error shape, not axum's plain-text rejection
- [x] `GET /auth/me` uses the `AuthUser` extractor and returns
      `{ id, login, avatarUrl, role }`. It reads the `users` row (the JWT
      carries neither `login` nor `avatarUrl`), so unlike other `AuthUser`
      routes it sees a deletion at once: a tombstoned row is `401`
- [x] `GET /auth/me` without a token, or with an expired one, returns `401`
- [x] Both routes registered under `/api` — the version segment is retired in
      v2 ([`api-rewrite`](../../api-rewrite/spec.md)), and this feature builds on
      the v2 branch, so it never sees `/api/v1`
- [x] Tests: refresh rotates and invalidates the previous token; an expired
      token is rejected; a deleted user is rejected; `me` returns the
      authenticated user; all reject unauthenticated calls

## Comments

**2026-11-17 — built.** The rotation is a single `UPDATE … FROM users …
RETURNING`; under READ COMMITTED a racing second refresh waits on the row
lock, re-evaluates its `WHERE` against the new hash and matches nothing, so
no explicit lock is taken (a test races two). Every refusal is the same
`401 invalid refresh token`: unknown, expired, rotated and deleted all leave
the client one move, and distinguishing them would only inform a thief.
`/me` returns the row's role, not the JWT's. The `#[expect(dead_code)]`
markers from issue 02 fired as unfulfilled the moment these routes mounted,
and are gone. The contract's §4 and §6 are updated here rather than left
for issue 05.

**2026-09-16 — the refresh token stays in client storage, and it is not a
preference.**

The obvious hardening is an httpOnly `Secure` cookie so XSS cannot read the
credential. It is unavailable on the current hosts. The frontend is
`https://maiscope.pages.dev` and the API is `https://maiscope-api.fly.dev`
(`fly.toml:46`, `deploy-web.yml:27`), and **`pages.dev` and `fly.dev` are both
Public Suffix List entries** — so a cookie set by the API while the user is on
the frontend is a third-party cookie. Safari blocks it by default and Chrome is
restricting it. Not harder: unavailable.

So the token stays in the response body, the client stores it, and requests use
`Authorization: Bearer`. Consequences accepted:

- XSS on the frontend origin is credential theft, not just defacement. A strict
  CSP on the Pages deploy is the mitigation that remains available.
- Rotation on every use means a stolen token is usable once and the theft
  surfaces as an unexpected logout — but rotation bounds nothing on its own,
  which is why `user_sessions.expires_at` exists. **30 days.**

A custom domain (`maiscope.dev` + `api.maiscope.dev`) would make same-site
cookies possible and is deliberately **optional**: `CORS_ALLOWED_ORIGINS` is
already config-driven, so adopting one later needs no code change, and buying
one now would add infra work to the branch whose length is the milestone's
largest risk.

