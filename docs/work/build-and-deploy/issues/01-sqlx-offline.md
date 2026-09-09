# 01 — `cargo sqlx prepare`: commit `.sqlx/`, build with `SQLX_OFFLINE`

**What to build:** The server uses **62 compile-time `sqlx::query!`/`query_as!`
macros** and there is no `.sqlx/` directory anywhere in the repo. So `cargo build`
currently **requires a reachable `DATABASE_URL`** — which a `docker build` does
not have. This blocks containerisation outright.

Generate the offline query cache, commit it, and build with `SQLX_OFFLINE=true`.
CI runs `cargo sqlx prepare --check` against a service Postgres so that changing
a query without re-preparing fails the build with a clear message instead of at
runtime.

**Blocked by:** `server-restructure/issues/04-split-modules.md` (do it once the SQL has finished moving between modules)

**Status:** todo

- [ ] `cargo sqlx prepare --workspace` run against a migrated local DB
- [ ] `.sqlx/` committed (it is generated state, and that is intentional)
- [ ] `SQLX_OFFLINE=true` set for the Docker build (issue 02)
- [ ] Local dev still works online without the env var
- [ ] `cargo sqlx prepare --check` wired into CI (issue 04)
- [ ] Contributor note in README/CLAUDE.md: re-run `prepare` after any query change
