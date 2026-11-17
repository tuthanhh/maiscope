# 06 — `DELETE /api/auth/me` (tombstone the account)

**What to build:** the endpoint behind "delete my account". It strips identity
from the row; it does not remove the row, and it does not touch uploads.

Decided 2026-09-16 — reasoning and the two rejected alternatives are in issue
01's comments, and the rule is cross-cutting rule 6 in
[`production-v2`](../../production-v2/spec.md).

**Blocked by:** 02 — needs the `AuthUser` extractor; 01 for `deleted_at`.

**Status:** todo

- [ ] `DELETE /api/auth/me`, `user+`, acting on `AuthUser.id` — never on an id
      taken from the path or body
- [ ] Sets `deleted_at`, nulls `login` and `avatar_url`, resets `role` to
      `user`, sets `updated_at` (`users_tombstone_shape` rejects a partial
      tombstone), and deletes every `user_sessions` row of the user, in one
      transaction
- [ ] **Does not clear `github_id`.** Without it, a re-login mints a second
      account and the tombstone is unreachable
- [ ] Does not touch `community_songs`, `community_charts` or
      `community_reports`. Published charts stay up, attributed to a deleted user
- [ ] Idempotent: deleting an already-deleted account is a success, not a `404`
- [ ] The outstanding JWT keeps working until it expires — up to 15 minutes.
      Accepted, not fixed here; see `production-v2`'s *Accepted, not solved*
- [ ] `GET /api/auth/me` for a deleted user returns `401`, not a half-empty user.
      Not a contradiction of the line above: `/me` reads the row for `login`
      and `avatarUrl` anyway (issue 04); routes that need only the id honour
      the JWT until it expires
- [ ] Anything rendering an uploader shows a fixed placeholder for a null
      `login` — one place, not per screen
- [ ] Tests: the row survives, `login`/`avatar_url` are null, `github_id` is
      intact, uploads are untouched, refresh is rejected afterwards, and a
      second delete succeeds

> This is identity erasure, not erasure. Whatever UI copy sits in front of it
> must say so — a user who reads "delete" and expects their uploads to vanish
> has been misled by us, not by the feature.
