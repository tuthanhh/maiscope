# 05 — Update the API contract §4 and the schema doc

> **Reconstructed 2026-09-09** from the plan at
> `git show 301b167:docs/superpowers/plans/2026-09-07-server-auth-github-oauth.md`
> (removed from the working tree since; recoverable from that commit), Task 5,
> after the original ticket file was lost during a botched directory move.
>
> **Path note:** the plan referenced `apps/server/docs/`. Those documents now live
> at `docs/reference/`. Use the current paths.

**What to build:** Bring the contract and schema documents in line with what
tickets 01–04 actually built, in the same change — the repository's doc-sync rule.

**Blocked by:** 01, 02, 03, 04

**Status:** done

- [x] `docs/reference/api-contract.md` §4 callback response updated to
      `{ "token": "<jwt>", "refreshToken": "<opaque>", "user": { ... } }`, with a
      note explaining why it deviates from the earlier cookie-session draft
- [x] `POST /auth/refresh` documented: request `{ "refreshToken": "<opaque>" }`,
      response `{ "token", "refreshToken" }`, and that refresh tokens rotate on use
- [x] JWT lifetime (15 minutes) stated
- [x] `401` conditions documented for every §4 endpoint
- [x] `docs/reference/schema.md` gains the `users` and `user_sessions` tables:
      every column, the constraints, and a note that `refresh_token_hash` holds a
      SHA-256 digest, never a usable token
- [x] No documented endpoint lacks an implementation, and no implemented endpoint
      is missing from the contract

## Comments

**2026-11-17 — closed as an audit.** The repository's doc-sync rule put each
piece in the commit that changed the behaviour, so by the time this ticket
came up there was nothing left to write, only to check:

| Item | Where |
|---|---|
| Callback response, and why not a cookie | contract §4, sign-in flow and *Tokens live in client storage* |
| Refresh request, response, rotation | contract §4 (issue 04) |
| 15-minute JWT | contract §4, *The JWT is authoritative* |
| `401` per endpoint | contract §4: login/callback have none (`400`/`502` instead); refresh, `me` and `DELETE me` each state theirs; the *401 responses* paragraph lists every message |
| `users`, `user_sessions` | `schema.md`, *Account tables* (issues 01, 03) |
| Parity | five routes mounted in `routes/auth.rs`, five documented in §4 |

`scripts/check-doc-links.mjs` passes. Left alone, outside this feature:
`schema.md`'s *Planned tables* still lists `contributions` and `audit_log`
from the superseded design; `community-charts` 07 rewrites that.
