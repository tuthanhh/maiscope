# 02 — Server: `config.rs`, environment-driven, no `unwrap`

**What to build:** A `Config` struct loaded once at startup from the environment,
replacing the ad-hoc `std::env::var("DATABASE_URL").unwrap()` at `main.rs:69` and
the hardcoded `0.0.0.0:3000` bind at `main.rs:95`. Missing or malformed
configuration must fail with a message that names the variable, not a bare panic
payload.

Fly injects `PORT`; Cloudflare Pages origins are not known at compile time; the
log filter differs between local and prod. All of it belongs here.

**Blocked by:** None.

**Status:** todo

- [ ] `Config` struct: `database_url`, `port`, `cors_allowed_origins` (list),
      `log_filter`
- [ ] Parsed once in `main`, returns `Result` with a variable-naming error message
- [ ] `port` defaults to 3000 when unset so local dev is unchanged
- [ ] `dotenvy` still loaded first so local `.env` keeps working
- [ ] `.env.example` updated with every new key and a comment per key
- [ ] No `unwrap`/`expect` on environment access anywhere in `main.rs`
- [ ] Existing `#[sqlx::test]` tests still pass untouched
