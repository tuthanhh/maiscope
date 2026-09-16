# 06 — Reports

**What to build:** the queue that replaces the approval queue. Live-on-upload
moves review after the fact, so this is where problems surface.

**Blocked by:** 01 — tables migration;
`server-auth-github-oauth/issues/02-jwt-and-authuser-extractor.md`.

**Status:** todo

- [ ] `POST /community/reports`, `user+` — target a song or a single chart, with
      a reason
- [ ] `GET /community/reports?resolved=`, moderator+, paginated with a `total`
      field
- [ ] `POST /community/reports/{id}/resolve`, moderator+, records who and what
      was decided
- [ ] One open report per reporter per target — resubmitting is not a second
      report
- [ ] Resolving does not take anything down; that is ticket 05. Keeping them
      separate means "reviewed and kept" is a recordable outcome
- [ ] Tests: duplicate report rejected, non-moderator cannot list, resolve is
      idempotent
