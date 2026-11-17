# 03 — GitHub OAuth login + callback handlers

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit), Task 3,
> after the original ticket file was lost during a botched directory move.

**What to build:** The two handlers that turn a GitHub identity into a maiscope
session: `GET /api/auth/github/login` (302 to GitHub) and
`GET /api/auth/github/callback?code=...` (exchange the code, upsert the user,
issue tokens).

**GitHub redirects to the frontend, not to the API** (maintainer decision,
2026-11-17). The SPA and the API sit on different registrable domains, so a
JSON body returned to a browser navigation on the API origin is unreadable by
the SPA. The flow:

1. The SPA generates a random `state`, keeps it in `sessionStorage`, and
   navigates to `/api/auth/github/login?state=<state>`.
2. The API 302s to GitHub's authorize URL carrying that `state` and
   `GITHUB_REDIRECT_URL`, which is the SPA's `/auth/callback` route.
3. GitHub sends the browser to the SPA with `code` and `state`. The SPA
   compares `state` with its stored copy, which is the login-CSRF defence: it
   binds the response to the browser that started the flow.
4. The SPA `fetch`es `/api/auth/github/callback?code=...` and receives the
   JSON below.

The server keeps no OAuth state. Rejected: the API as redirect target with
tokens in a URL fragment (a 30-day credential in browser history), and with a
one-time exchange code (a new store and endpoint for the same result).

**Contract deviation, stated up front:** the callback returns
`{ "token", "refreshToken", "user" }`, not the `{ "token", "user" }` the contract
currently documents. The contract's shape implies a cookie-based session, which
does not fit a client that cannot rely on an ambient cookie jar. `refreshToken` is
an additive field. Ticket 05 updates the contract to match in the same feature.

**Blocked by:** 02 (needs `issue_jwt`, `generate_refresh_token`, `hash_refresh_token`)

**Status:** todo

- [ ] `UserRow` added to `types.rs`, `Serialize`, `#[serde(rename_all = "camelCase")]`,
      with `id` serialised as a string per the contract
- [ ] `GET /api/auth/github/login?state=` redirects (302) to GitHub's authorize
      URL with the configured client id, `GITHUB_REDIRECT_URL` and the given
      `state`; a missing or malformed `state` is `400`, never a redirect without
      one
- [ ] `GET /api/auth/github/callback?code=` exchanges the code for a GitHub
      access token and fetches the profile; the OAuth crate is handed the
      server's reqwest 0.13 (it is built without its own client)
- [ ] User upserted by `github_id`: insert on first login, update `login` and
      `avatar_url` on subsequent ones. `role` is never overwritten by a login.
- [ ] **A tombstoned account reactivates.** If the matched row has `deleted_at`
      set, clear it and repopulate `login` and `avatar_url` from the profile.
      This is the only path that reactivates — refresh does not (issue 04).
      `role` is still not overwritten, and after a deletion it is `user`
      (issue 01's comments)
- [ ] Every `UPDATE` sets `updated_at = now()`
- [ ] Refresh token generated, hashed, stored in `users.refresh_token_hash`,
      with `refresh_token_expires_at` set to now + 30 days
- [ ] Response body: `{ "token": "<jwt>", "refreshToken": "<opaque>", "user": { ... } }`
- [ ] Failure paths return the contract error shape, never a raw OAuth error
- [ ] Client secret never logged, never returned
