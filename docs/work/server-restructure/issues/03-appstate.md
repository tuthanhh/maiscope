# 03 — Server: `AppState` instead of a bare `Pool<Postgres>`

**What to build:** Router state is currently the raw pool
(`main.rs:93` `.with_state(pool)`), so every handler extracts
`State(pool): State<Pool<Postgres>>`. Adding anything else — config, an HTTP
client, a cache — would mean editing all eight handler signatures.

Introduce `AppState { pool, config }`, cheap to clone (`Arc` the config), and
implement `FromRef` so handlers that only want the pool can keep asking for one.

**Blocked by:** 01 (needs `Config`)

**Status:** done

- [x] `state.rs` with `AppState { pool: PgPool, config: Arc<Config> }`, `Clone`
- [x] `impl FromRef<AppState> for PgPool` so pool-only handlers stay unchanged
- [x] `.with_state(AppState { .. })` in the router
- [x] Pool sizing moved to config — note Neon's free-tier connection ceiling;
      the current `max_connections(4)` (`main.rs:72`) was chosen for local docker
- [x] `#[sqlx::test]` handler tests updated to construct `AppState`, still green
      (see Comments — they didn't need to change)

## Comments

`FromRef<AppState> for PgPool` means every handler still asks for
`State<Pool<Postgres>>` unchanged — none of the eight signatures moved to
`State<AppState>`. The `#[sqlx::test]` tests call handlers directly with
`AxumState(pool)`, bypassing the router entirely, so they were never touched;
the checklist item is satisfied by construction rather than by an edit.

`Config` gained `database_max_connections` (`DATABASE_MAX_CONNECTIONS`, default
4, matching the value that was hardcoded) so pool sizing is configurable
without another ticket touching `Config` again.

Caught in review: the initial wiring read `state.config.port` *after*
`state` had already been moved into `.with_state(state)` — a use-after-move
that wouldn't compile. Fixed by capturing `port` before the move.
