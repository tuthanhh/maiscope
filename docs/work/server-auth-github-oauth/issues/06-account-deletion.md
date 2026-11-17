# 06 — `DELETE /api/auth/me` (tombstone the account)

**What to build:** the endpoint behind "delete my account". It strips identity
from the row; it does not remove the row, and it does not touch uploads.

Decided 2026-09-16 — reasoning and the two rejected alternatives are in issue
01's comments, and the rule is cross-cutting rule 6 in
[`production-v2`](../../production-v2/spec.md).

**Blocked by:** 02 — needs the `AuthUser` extractor; 01 for `deleted_at`.

**Status:** done (two items carried)

- [x] `DELETE /api/auth/me`, `user+`, acting on `AuthUser.id` — never on an id
      taken from the path or body
- [x] Sets `deleted_at`, nulls `login` and `avatar_url`, resets `role` to
      `user`, sets `updated_at` (`users_tombstone_shape` rejects a partial
      tombstone), and deletes every `user_sessions` row of the user, in one
      transaction
- [x] **Does not clear `github_id`.** Without it, a re-login mints a second
      account and the tombstone is unreachable
- [ ] **Carried to [`community-charts` 01](../../community-charts/issues/01-community-tables-migration.md).**
      Does not touch `community_songs`, `community_charts` or
      `community_reports`. Published charts stay up, attributed to a deleted
      user. True by construction today — the tables do not exist and the
      tombstone writes only `users` and `user_sessions` — but the test needs the
      tables, so it belongs with their migration
- [x] Idempotent: deleting an already-deleted account is a success, not a `404`
- [x] The outstanding JWT keeps working until it expires — up to 15 minutes.
      Accepted, not fixed here; see `production-v2`'s *Accepted, not solved*
- [x] `GET /api/auth/me` for a deleted user returns `401`, not a half-empty user.
      Not a contradiction of the line above: `/me` reads the row for `login`
      and `avatarUrl` anyway (issue 04); routes that need only the id honour
      the JWT until it expires
- [ ] **Carried to [`frontend-redesign` 05](../../frontend-redesign/issues/05-community-screens.md).**
      Anything rendering an uploader shows a fixed placeholder for a null
      `login` — one place, not per screen. A UI rule; no server code renders
      an uploader
- [x] Tests: the row survives, `login`/`avatar_url` are null, `github_id` is
      intact, refresh is rejected afterwards, and a second delete succeeds
      ("uploads are untouched" goes with the carried item above)

> This is identity erasure, not erasure. Whatever UI copy sits in front of it
> must say so — a user who reads "delete" and expects their uploads to vanish
> has been misled by us, not by the feature.

## Comments

**2026-11-17 — built.** `204 No Content`. The tombstone is one transaction:
an `UPDATE` of `users` (keeping the first `deleted_at` on a repeat) and a
`DELETE` of the user's `user_sessions`. A refresh racing the deletion cannot
outlive it: if the deletion commits first, the refresh's `UPDATE` sees the
tombstone or finds its session gone; if the refresh commits first, the
deletion's `DELETE` waits for the rotated row and removes it. Either way the
only survivor is a JWT of at most 15 minutes, which is the accepted lag.

**2026-11-17 — review.** Applied: a repeat deletion is a true no-op
(`updated_at` no longer moves); the contract says the repeat is `204` only
while the token verifies; a test forces a refresh to wait on a deletion's
open transaction and finds it refused with no session left; the assembled
router refuses an anonymous `DELETE` and preflights it.

Accepted, under the existing 15-minute decision: a JWT issued before a
deletion still verifies after the user signs in again, so within that
window it can delete the reactivated account a second time. Closing it means
a per-user "identity since" timestamp that `AuthUser` or each destructive
handler compares the token's `iat` against — a database read the spec chose
not to take for every request. Same person in the ordinary case; a stolen
JWT could delete the account anyway before the first deletion.
