# 02 — JWT issue/verify + `AuthUser` extractor

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit), Task 2,
> after the original ticket file was lost during a botched directory move.

**What to build:** `apps/server/src/auth.rs` — the module every later auth and
contributions handler depends on. It owns JWT issue/verify, refresh-token
generation and hashing, and the Axum extractor that turns an
`Authorization: Bearer <jwt>` header into a verified user and role.

This is the seam the Contributions tier gates its writes on, so its signatures are
load-bearing beyond this feature.

**Blocked by:** 01 (needs the `users` table and `JWT_SECRET`)

**Status:** done

- [x] `pub struct Claims { pub sub: String, pub role: Role, pub iat: u64, pub exp: u64 }`
      — `sub` is the `users.id` UUID, hyphenated
- [x] `pub enum Role { User, Moderator, Admin }`, serialised lowercase
- [x] `pub fn issue_jwt(secret: &str, user_id: Uuid, role: Role) -> Result<String, jsonwebtoken::errors::Error>`
      — 15-minute expiry (`ACCESS_TOKEN_TTL`)
- [x] `pub fn verify_jwt(secret: &str, token: &str) -> Result<AuthUser, TokenError>`
      — HS256 pinned, no leeway, `exp` and `sub` required
- [x] `pub fn generate_refresh_token() -> String` — 32 random bytes, hex; **not** a JWT
- [x] `pub fn hash_refresh_token(token: &str) -> String` — SHA-256 hex digest, the
      value stored in `users.refresh_token_hash`
- [x] `pub struct AuthUser { pub id: Uuid, pub role: Role }` implementing
      `FromRequestParts<AppState>`
- [x] `AuthUser` rejects with `401` on a missing, malformed, invalid, or expired token
      — `AppError::Unauthorized` (no credential, `WWW-Authenticate: Bearer`) or
      `AppError::InvalidToken` (refused, `Bearer error="invalid_token"`)
- [x] `JWT_SECRET` is a required `Config` field, at least 32 bytes, held in a
      `Secret` whose `Debug` prints `Secret(***)`
- [x] `mod auth;` declared in `main.rs`
- [x] Unit tests: a round-tripped JWT verifies; a tampered one does not; an expired
      one is rejected

## Comments

**2026-11-17 — built; departures from the September plan.**

- **Ids are UUIDs** (issue 01). `Claims.sub` is a `String` because RFC 7519
  makes `sub` a string and the `uuid` crate is built without serde here;
  `verify_jwt` parses it, and a subject that is not a UUID is an invalid token.
- **`role` is an enum.** A token naming any other role fails to decode and is
  rejected, rather than reaching a handler as an unknown string.
- **The secret is passed in**, from `Config`, not read from the environment
  inside `auth.rs`. The server refuses to start without one: a copied
  `.env.example` would otherwise boot with an empty HS256 key.
- **The 401 says which of missing, expired or invalid**, so a client knows
  whether a refresh can help. It never says why a token is invalid.
- **Nothing calls this yet.** `mod auth` and `AppError::Unauthorized` carry
  `#[expect(dead_code)]` outside tests. An `expect` that stops being needed is
  itself a build error, so the markers are removed by whichever ticket gives
  the last item a caller (04, `/auth/me`) rather than lingering.

**Deploy prerequisite:** production needs a `JWT_SECRET` Fly secret before
the v2 branch deploys, or the machine fails at startup with
`JWT_SECRET is not set`.

**2026-11-17 — review.** Applied: a token is expired from the second of its
`exp`, not one second after (jsonwebtoken's own check is `exp < now`);
tests now pin the algorithm (HS384/HS512 under the right secret are refused)
and the required claims; a refused token's challenge carries
`error="invalid_token"` per RFC 6750 §3.1; `Secret` lost `PartialEq`. The
32-byte floor is documented as catching an empty or truncated value, not a
weak one.
