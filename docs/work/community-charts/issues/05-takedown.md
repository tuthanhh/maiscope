# 05 — DELETE /community/songs/{id}

**What to build:** takedown, by the uploader or a moderator.

Soft delete — `status = 'removed'` — not a row delete. With no review step the
takedown record *is* the moderation history, and a hard delete leaves nothing to
show that a report was acted on.

**Blocked by:** 01 — tables migration;
`server-auth-github-oauth/issues/02-jwt-and-authuser-extractor.md`.

**Status:** todo

- [ ] Uploader may remove their own; moderator+ may remove any
- [ ] A user removing someone else's gets `404`, not `403` — same
      existence-leak rule the superseded contributions design used
- [ ] Sets `status = 'removed'`; charts stay in place via the cascade being
      unused here
- [ ] Removal is idempotent
- [ ] Tests: uploader removes own, moderator removes another's, stranger gets
      404, double-remove is not an error
