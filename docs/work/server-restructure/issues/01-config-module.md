# 01 — Server: `config.rs`, environment-driven, no `unwrap`

**What to build:** A `Config` struct loaded once at startup from the environment,
replacing the ad-hoc `std::env::var("DATABASE_URL").unwrap()` at `main.rs:69` and
the hardcoded `0.0.0.0:3000` bind at `main.rs:95`. Missing or malformed
configuration must fail with a message that names the variable, not a bare panic
payload.

Fly injects `PORT`; Cloudflare Pages origins are not known at compile time; the
log filter differs between local and prod. All of it belongs here.

**Blocked by:** None.

**Status:** done

- [x] `Config` struct: `database_url`, `port`, `cors_allowed_origins` (list),
      `log_filter`
- [x] Parsed once in `main`, returns `Result` with a variable-naming error message
- [x] `port` defaults to 3000 when unset so local dev is unchanged
- [x] `dotenvy` still loaded first so local `.env` keeps working
- [x] `.env.example` updated with every new key and a comment per key
- [x] No `unwrap`/`expect` on environment access anywhere in `main.rs`
- [x] Existing `#[sqlx::test]` tests still pass untouched

## Comments

`cors_allowed_origins` parses as comma-separated exact `Origin` values; unset/empty
means no allowlist configured. Ticket 06 decides what that implies (fail closed vs.
permissive fallback for local dev) and turns the list into a `CorsLayer`. Field is
currently unread (`dead_code` warning) until 05/06 consume it — expected.

Parsing logic (`parse_port`, `parse_cors_origins`) lives in pure functions with unit
tests. `required`/`optional` also have tests, but guard the process env with a
private var name + mutex — `Config::from_env` shares a test binary with the
`#[sqlx::test]` handler tests, which read `DATABASE_URL` live, so tests never
touch the real config vars.
