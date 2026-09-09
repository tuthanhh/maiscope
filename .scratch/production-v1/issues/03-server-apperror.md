# 03 — Server: one `AppError` implementing `IntoResponse`

**What to build:** Replace the ad-hoc error tuples with a single error type.
Today every handler hand-assembles `(StatusCode, Json<Value>)` via the helpers
`db_error` (`main.rs:109`) and `not_found` (`main.rs:116`), so the error contract
is enforced by convention rather than by the compiler.

`AppError` becomes an enum (`Database`, `NotFound { kind, key }`, `BadRequest`,
`SnapshotRequired` for `/sync/delta`'s `409`) with `impl IntoResponse` producing
the contract error shape in exactly one place. Handlers return
`Result<Json<T>, AppError>` and use `?`.

**Blocked by:** None.

**Status:** todo

- [ ] `error.rs` with `AppError` enum covering every status the contract emits
- [ ] `impl IntoResponse for AppError` — single place that builds the JSON body
- [ ] `impl From<sqlx::Error> for AppError` so handlers can `?` on queries
- [ ] `409 snapshot_required` for `/sync/delta` preserved exactly (contract §3)
- [ ] `db_error` and `not_found` helpers deleted
- [ ] Every handler signature returns `Result<_, AppError>`
- [ ] Error responses byte-compatible with today's shape — verified by the
      existing 404 tests (`main.rs:523`, `main.rs:552`)
- [ ] Internal error detail logged, **not** returned to the client
