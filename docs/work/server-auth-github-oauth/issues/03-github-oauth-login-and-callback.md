# 03 — GitHub OAuth login + callback handlers

> **Reconstructed 2026-09-09** from `docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> Task 3 after the original ticket file was lost during a botched directory move.

**What to build:** The two handlers that turn a GitHub identity into a maiscope
session: `GET /auth/github/login` (302 to GitHub) and
`GET /auth/github/callback?code=...` (exchange the code, upsert the user, issue
tokens).

**Contract deviation, stated up front:** the callback returns
`{ "token", "refreshToken", "user" }`, not the `{ "token", "user" }` the contract
currently documents. The contract's shape implies a cookie-based session, which
does not fit a client that cannot rely on an ambient cookie jar. `refreshToken` is
an additive field. Ticket 05 updates the contract to match in the same feature.

**Blocked by:** 02 (needs `issue_jwt`, `generate_refresh_token`, `hash_refresh_token`)

**Status:** ready-for-agent

- [ ] `UserRow` added to `types.rs`, `Serialize`, `#[serde(rename_all = "camelCase")]`,
      with `id` serialised as a string per the contract
- [ ] `GET /auth/github/login` redirects (302) to GitHub's authorize URL with the
      configured client id and callback
- [ ] CSRF `state` parameter generated, and verified on callback
- [ ] `GET /auth/github/callback` exchanges the code for a GitHub access token and
      fetches the profile
- [ ] User upserted by `github_id`: insert on first login, update `login` and
      `avatar_url` on subsequent ones. `role` is never overwritten by a login.
- [ ] Refresh token generated, hashed, stored in `users.refresh_token_hash`
- [ ] Response body: `{ "token": "<jwt>", "refreshToken": "<opaque>", "user": { ... } }`
- [ ] Failure paths return the contract error shape, never a raw OAuth error
- [ ] Client secret never logged, never returned
