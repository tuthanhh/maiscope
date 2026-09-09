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

**Status:** todo

- [ ] `pub struct Claims { pub sub: i64, pub role: String, pub exp: usize }`
- [ ] `pub fn issue_jwt(user_id: i64, role: &str) -> Result<String, jsonwebtoken::errors::Error>`
      — 15-minute expiry
- [ ] `pub fn generate_refresh_token() -> String` — random opaque token, **not** a JWT
- [ ] `pub fn hash_refresh_token(token: &str) -> String` — SHA-256 hex digest, the
      value stored in `users.refresh_token_hash`
- [ ] `pub struct AuthUser { pub id: i64, pub role: String }` implementing
      `FromRequestParts`
- [ ] `AuthUser` rejects with `401` on a missing, malformed, invalid, or expired token
- [ ] `mod auth;` declared in `main.rs`
- [ ] Unit tests: a round-tripped JWT verifies; a tampered one does not; an expired
      one is rejected
