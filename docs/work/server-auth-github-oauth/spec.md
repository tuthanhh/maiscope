# Spec — GitHub OAuth accounts

**Status:** active
**Milestone:** v2.0
**Parent:** [`production-v2`](../production-v2/spec.md)

Give maiscope accounts, so a write endpoint has someone to attribute work to and
a takedown has someone to act against.

Backfilled 2026-09-16. This feature predates the umbrella and ran as tickets
only; several decisions since landed in ticket comments rather than anywhere a
reader would look first. This spec is that place.

## Problem

v2 makes maiscope writable. Every write needs an identity, and ADR-0013's
published-on-upload model needs that identity to persist after the fact —
reports and takedowns happen *after* content is live, so an anonymous upload is
one nobody can be held to.

GitHub OAuth rather than passwords: the audience already has GitHub accounts,
and not storing a password is one entire class of breach the project never has
to plan for.

## Scope

| # | Ticket | Note |
|---|---|---|
| 01 | [`users` table + dependencies](issues/01-users-table-migration.md) | Also records the deletion design |
| 02 | [JWT issue/verify + `AuthUser`](issues/02-jwt-and-authuser-extractor.md) | The seam every write gates on |
| 03 | [OAuth login and callback](issues/03-github-oauth-login-and-callback.md) | Owns tombstone reactivation |
| 04 | [Refresh and me](issues/04-refresh-and-me.md) | Rotation, 30-day expiry |
| 06 | [Account deletion](issues/06-account-deletion.md) | `DELETE /api/auth/me` |
| 05 | [Contract and schema docs](issues/05-update-contract-and-schema-docs.md) | Last — describes what shipped |

Ticket numbers are creation order; 06 was added after 05 existed.

## Decisions that constrain this work

- **No password, no email.** Stored: `github_id`, `login`, `avatar_url`, `role`,
  a refresh-token *hash*, timestamps. v1 deliberately held no PII, and ending
  that is a decision worth making narrowly rather than by default.
- **Refresh tokens are opaque and hashed at rest**, never JWTs. A database leak
  must not hand out working credentials.
- **One session per device.** Refresh tokens live in `user_sessions`, a row
  each, so signing in on a phone does not sign the desktop out. Chosen over a
  single column on `users` before any handler existed (issue 01's comments).
- **They rotate on every use and expire after 30 days.** Rotation makes a stolen
  token single-use and surfaces the theft as an unexpected logout; the expiry is
  what bounds a stolen token that is simply never used.
- **They live in client storage, not a cookie** — not a preference. The frontend
  and API sit on different registrable domains and both `pages.dev` and
  `fly.dev` are Public Suffix List entries, so an API-set cookie is third-party
  and Safari blocks it outright. Reasoning and the accepted XSS consequence are
  in issue 04's comments.
- **Deleting an account is a tombstone.** Cross-cutting rule 6 in the umbrella;
  mechanics and the two rejected alternatives in issue 01's comments.
- **The JWT is authoritative for 15 minutes.** `AuthUser` reads no database, so
  a role change or deletion takes up to one token lifetime to bite. Accepted
  knowingly — see the umbrella's *Accepted, not solved*.
- **This feature builds on the v2 branch**, so its routes are `/api/...` from
  the start and never see `/api/v1`.

## Out of scope

Bans and suspensions. `role` stays `user | moderator | admin`, and the only
tools against a repeat abuser are per-song takedown and the reports queue —
deferred deliberately, with the migration cost of adding `banned_at` later
measured as negligible. **When it lands, reactivation must clear `deleted_at`
and not `banned_at`.**

Per-user rate limiting. Writes reuse the existing global per-IP limiter.

Any write endpoint. Those are [`community-charts`](../community-charts/spec.md);
this feature only builds the gate they sit behind.

## Done when

A GitHub login returns a JWT and a refresh token, `AuthUser` rejects anything
malformed or expired, refresh rotates, a deleted account keeps its uploads and
loses its identity, re-authenticating restores it, and §4 of
[`api-contract.md`](../../reference/api-contract.md) describes what actually
shipped.
