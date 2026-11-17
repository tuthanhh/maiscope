# 05 — `GET /api/songs/{id}/sheets/{type}/{difficulty}/chart`

**What to build:** the chart text for the engine, at the sheet's own address,
with plain-text errors that go through `AppError` (spec, *Settled
2026-09-16*). v1's handler returns `e.to_string()` of a raw `sqlx::Error` to
the client; the documented exception to the JSON envelope became an
exception to redaction too. This closes it by construction.

**Blocked by:** 04 (same path parsing and sheet lookup).

**Status:** done

- [x] `200 text/plain; charset=utf-8` with the simai text
- [x] `AppError` gains a plain-text rendering — same status and redaction as
      the JSON one, body is the message only — and this handler returns it.
      Database errors render as `internal error`, never the sqlx text
- [x] No chart row → `404`; a row with no inline content → `501`, as v1
- [x] Path errors (bad id, unknown sheet) use the plain-text rendering too: the
      engine reads this body as text whatever the status
- [x] Tests: the text round-trips; a database error's body contains no sqlx
      text (force one, as `routes/auth.rs` forces a rollback); 404 and 501
- [x] Contract: the route, and §6's exception reworded — plain text, same
      redaction

## Comments

**2026-11-17 — built.** `AppError` now renders once into shared parts
(status, code, message, challenge header, and the error log), and both the
JSON body and `PlainText(AppError)` are built from them — so plain text
cannot differ in status or redaction. `PlainText` converts from `sqlx::Error`
only through `AppError::Database`, so `?` on a query in this handler cannot
leak. The chart lookup is one statement distinguishing no sheet, no chart
row, blob-only and inline. A test renames `charts` mid-test to force a
database error and finds the body is exactly `internal error`.
