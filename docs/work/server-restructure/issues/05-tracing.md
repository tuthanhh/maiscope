# 05 — Server: `tracing` + request spans

**What to build:** The server has no observability at all — no `tracing`
dependency, no `TraceLayer`, one `println!` at startup (`main.rs:99`). In
production that means a failure produces nothing to look at.

Add `tracing` + `tracing-subscriber` emitting JSON to stdout (Fly captures
stdout), plus `tower_http::trace::TraceLayer` for per-request spans. Filter from
config so local dev stays human-readable.

**Blocked by:** 01 (log filter comes from `Config`)

**Status:** todo

- [ ] `tracing`, `tracing-subscriber` (`env-filter`, `json`) deps added
- [ ] `tower-http` gains the `trace` feature
- [ ] JSON formatter in prod, pretty formatter locally, switched by config
- [ ] `TraceLayer` on the router: method, path, status, latency
- [ ] Startup log includes bind address, resolved config (**never** the
      `DATABASE_URL` password), and the migration/schema version if available
- [ ] All `println!` replaced with `tracing` macros
- [ ] `AppError::Database` logs the underlying `sqlx::Error` at `error` level
      while still returning an opaque body to the client (issue 02)
- [ ] Note in the ticket: Fly log retention without a drain is recent-only;
      acceptable at v1
