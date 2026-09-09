# 05 — Update the API contract §4 and the schema doc

> **Reconstructed 2026-09-09** from `docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> Task 5 after the original ticket file was lost during a botched directory move.
>
> **Path note:** the plan referenced `apps/server/docs/`. Those documents now live
> at `docs/reference/`. Use the current paths.

**What to build:** Bring the contract and schema documents in line with what
tickets 01–04 actually built, in the same change — the repository's doc-sync rule.

**Blocked by:** 01, 02, 03, 04

**Status:** ready-for-agent

- [ ] `docs/reference/api-contract.md` §4 callback response updated to
      `{ "token": "<jwt>", "refreshToken": "<opaque>", "user": { ... } }`, with a
      note explaining why it deviates from the earlier cookie-session draft
- [ ] `POST /auth/refresh` documented: request `{ "refreshToken": "<opaque>" }`,
      response `{ "token", "refreshToken" }`, and that refresh tokens rotate on use
- [ ] JWT lifetime (15 minutes) stated
- [ ] `401` conditions documented for every §4 endpoint
- [ ] `docs/reference/schema.md` gains the `users` table: every column, the role
      constraint, and a note that `refresh_token_hash` holds a SHA-256 digest, never
      a usable token
- [ ] No documented endpoint lacks an implementation, and no implemented endpoint
      is missing from the contract
