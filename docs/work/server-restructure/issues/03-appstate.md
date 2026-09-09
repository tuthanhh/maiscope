# 03 — Server: `AppState` instead of a bare `Pool<Postgres>`

**What to build:** Router state is currently the raw pool
(`main.rs:93` `.with_state(pool)`), so every handler extracts
`State(pool): State<Pool<Postgres>>`. Adding anything else — config, an HTTP
client, a cache — would mean editing all eight handler signatures.

Introduce `AppState { pool, config }`, cheap to clone (`Arc` the config), and
implement `FromRef` so handlers that only want the pool can keep asking for one.

**Blocked by:** 01 (needs `Config`)

**Status:** todo

- [ ] `state.rs` with `AppState { pool: PgPool, config: Arc<Config> }`, `Clone`
- [ ] `impl FromRef<AppState> for PgPool` so pool-only handlers stay unchanged
- [ ] `.with_state(AppState { .. })` in the router
- [ ] Pool sizing moved to config — note Neon's free-tier connection ceiling;
      the current `max_connections(4)` (`main.rs:72`) was chosen for local docker
- [ ] `#[sqlx::test]` handler tests updated to construct `AppState`, still green
