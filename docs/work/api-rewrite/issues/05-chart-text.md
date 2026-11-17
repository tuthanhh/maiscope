# 05 — `GET /api/songs/{id}/sheets/{type}/{difficulty}/chart`

**What to build:** the chart text for the engine, at the sheet's own address,
with plain-text errors that go through `AppError` (spec, *Settled
2026-09-16*). v1's handler returns `e.to_string()` of a raw `sqlx::Error` to
the client; the documented exception to the JSON envelope became an
exception to redaction too. This closes it by construction.

**Blocked by:** 04 (same path parsing and sheet lookup).

**Status:** todo

- [ ] `200 text/plain; charset=utf-8` with the simai text
- [ ] `AppError` gains a plain-text rendering — same status and redaction as
      the JSON one, body is the message only — and this handler returns it.
      Database errors render as `internal error`, never the sqlx text
- [ ] No chart row → `404`; a row with no inline content → `501`, as v1
- [ ] Path errors (bad id, unknown sheet) use the plain-text rendering too: the
      engine reads this body as text whatever the status
- [ ] Tests: the text round-trips; a database error's body contains no sqlx
      text (force one, as `routes/auth.rs` forces a rollback); 404 and 501
- [ ] Contract: the route, and §6's exception reworded — plain text, same
      redaction
