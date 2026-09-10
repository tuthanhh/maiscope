# 05 — Server: `tracing` + request spans

**What to build:** The server has no observability at all — no `tracing`
dependency, no `TraceLayer`, one `println!` at startup (`main.rs:99`). In
production that means a failure produces nothing to look at.

Add `tracing` + `tracing-subscriber` emitting JSON to stdout (Fly captures
stdout), plus `tower_http::trace::TraceLayer` for per-request spans. Filter from
config so local dev stays human-readable.

**Blocked by:** 01 (log filter comes from `Config`)

**Status:** done

- [x] `tracing`, `tracing-subscriber` (`env-filter`, `json`) deps added
- [x] `tower-http` gains the `trace` feature
- [x] JSON formatter in prod, pretty formatter locally, switched by config
      (`Config.log_json`, env `LOG_JSON`, default `false`/pretty)
- [x] `TraceLayer` on the router: method, path, status, latency
- [x] Startup log includes bind address, resolved config (**never** the
      `DATABASE_URL` password), and the migration/schema version if available
- [x] All `println!` replaced with `tracing` macros (see Comments for the two
      deliberate exceptions)
- [x] `AppError::Database` logs the underlying `sqlx::Error` at `error` level
      while still returning an opaque body to the client (issue 02)
- [x] Note in the ticket: Fly log retention without a drain is recent-only;
      acceptable at v1 — noted below.

## Comments

**Fly log retention:** Fly does not persist stdout beyond a short rolling
window unless a log drain is configured. No drain exists at v1.0, so history
older than that window is gone. Acceptable for v1 — revisit if debugging a
past incident becomes a recurring need.

**Two `println!`/`eprintln!` sites intentionally left untouched:**
- `main.rs`'s `eprintln!("startup failed: {e}")` in `main()` — `run()`'s first
  fallible call is `Config::from_env()`, and the subscriber is built *from*
  `config.log_filter`/`config.log_json`. If config parsing itself fails,
  there is no subscriber yet to log through, so this one path has no tracing
  infrastructure available to it, ever.
- `apps/server/src/bin/{ingest,seed_songs,seed_chart}.rs` — one-shot CLI
  tools, not the long-running server process. "Fly captures stdout" and the
  JSON/pretty split are about the server binary; these scripts are out of
  scope.

**`schema_version`:** sqlx tracks every applied migration in
`_sqlx_migrations`; `SELECT MAX(version)` against it is a cheap, always-
available proxy for "what schema is this server running against" — included
in the startup log as `schema_version` (an `Option<i64>`, `None` only if the
table is somehow empty, which shouldn't happen in practice).

**`TraceLayer` gotcha (worth flagging for anyone touching this later):**
`tower_http::trace`'s `DefaultMakeSpan` and `DefaultOnResponse` *both*
default to `Level::DEBUG`, independently of each other. `config.rs`'s
`DEFAULT_LOG_FILTER` is `"info,tower_http=info"`, so both need bumping to
`Level::INFO` explicitly — missing one is a silent partial failure, not a
compile error. In particular, missing `make_span_with`'s level bump makes
`on_response`'s event fire with no span context, so `status`/`latency`
still show but `method`/`path` silently disappear. Caught by hand-inspecting
live log output in both pretty and JSON mode, not by a test — this repo
doesn't have log-content assertions and adding a snapshot test purely to
pin two magic-constant `.level()` calls to `Level::INFO` felt like more
machinery than the risk warranted; the comment above the `.layer(...)` call
is the safeguard instead.
