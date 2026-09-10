# 02 — Server: one `AppError` implementing `IntoResponse`

**What to build:** Replace the ad-hoc error tuples with a single error type.
Today every handler hand-assembles `(StatusCode, Json<Value>)` via the helpers
`db_error` (`main.rs:109`) and `not_found` (`main.rs:116`), so the error contract
is enforced by convention rather than by the compiler.

`AppError` becomes an enum (`Database`, `NotFound { kind, key }`, `BadRequest`,
`SnapshotRequired` for `/sync/delta`'s `409`) with `impl IntoResponse` producing
the contract error shape in exactly one place. Handlers return
`Result<Json<T>, AppError>` and use `?`.

**Blocked by:** None.

**Status:** done

- [x] `error.rs` with `AppError` enum covering every status the contract emits
- [x] `impl IntoResponse for AppError` — single place that builds the JSON body
- [x] `impl From<sqlx::Error> for AppError` so handlers can `?` on queries
- [x] `409 snapshot_required` for `/sync/delta` preserved exactly (contract §3)
- [x] `db_error` and `not_found` helpers deleted
- [x] Every handler signature returns `Result<_, AppError>` — except `get_chart`
      (see Comments)
- [x] Error responses byte-compatible with today's shape — verified by the
      existing 404 tests (`main.rs:523`, `main.rs:552`), live `curl` against
      `/songs/{id}` (404) and `/sync/delta` (409), and new `error::tests`
      asserting the exact JSON body per variant
- [x] Internal error detail logged, **not** returned to the client

## Comments

`get_chart` keeps its own `(StatusCode, String)` return, not `AppError` — contract
§2/§6 explicitly excepts it: plain-text errors, not the JSON shape. Converting it
would have been a contract break, not a fix.

The original `db_error` helper put `e.to_string()` straight in the response body,
which is exactly what this ticket's last checklist item says to stop doing. Fixed
as part of the same change: `AppError::Database` now returns an opaque message and
logs the real error via `eprintln!` (swap for `tracing::error!` lands in ticket 05).
