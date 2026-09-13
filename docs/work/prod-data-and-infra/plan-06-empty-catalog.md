# Empty-Catalog 500 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `GET /api/v1/catalog` returns an empty catalog instead of 500 when the database has schema but no rows.

**Architecture:** `queries/catalog.rs:fetch_update_time` uses `fetch_one` against `catalog_meta`, a singleton only `bin/ingest`/`sync_catalog` ever writes. On a freshly migrated database the table is empty, so `fetch_one` returns `RowNotFound` and the handler maps it to 500. Switch to `fetch_optional` and fall back to the `0000-00-00` sentinel that `apps/host/src/utils/data.ts:19` already produces and `apps/host/src/utils/filter.ts:80` already branches on.

**Tech Stack:** Rust 1.97 (edition 2024), sqlx 0.9 with Postgres, axum 0.8. Tests are `#[sqlx::test]`.

**Spec:** [`docs/work/prod-data-and-infra/issues/06-empty-catalog-500.md`](issues/06-empty-catalog-500.md)

## Global Constraints

- **`queries/catalog.rs` uses the `sqlx::query_scalar!` macro**, which is backed by the committed `.sqlx/` cache. Changing that SQL string requires regenerating the cache:
  ```sh
  set -a; . apps/server/.env; set +a
  cargo sqlx prepare --workspace -- -p server --all-targets
  ```
  Commit the resulting `.sqlx/` changes. CI runs the same command with `--check` and fails on a stale cache.
- The unseeded sentinel is exactly `0000-00-00`. It is not a new invention — match `apps/host/src/utils/data.ts:19`.
- The wire contract in `docs/reference/api-contract.md:59` says `"updateTime": "YYYY-MM-DD"`. It stays a string; only its unseeded value is being pinned. Update that line in the same change — doc sync is part of the ticket that changes behaviour (CLAUDE.md).
- Task ends green: `cargo fmt --check`, `cargo clippy -p server --all-targets -- -D warnings`, `cargo test -p server`.

---

### Task 1: `/catalog` serves an empty catalog on an unseeded database

**Files:**
- Modify: `apps/server/src/queries/catalog.rs:52-58`
- Modify: `docs/reference/api-contract.md:59`
- Test: `apps/server/src/routes/catalog.rs` (existing `#[cfg(test)]` module)

**Interfaces:**
- Consumes: nothing
- Produces: `fetch_update_time` keeps its signature `async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error>` — it now yields `"0000-00-00"` rather than erroring when `catalog_meta` is empty. Callers are unchanged.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `apps/server/src/routes/catalog.rs`. Match the existing tests' style in that file for building the router and issuing the request — read them first and follow the same pattern rather than inventing a new one.

```rust
    #[sqlx::test]
    async fn catalog_on_an_unseeded_database_is_empty_not_an_error(pool: PgPool) -> sqlx::Result<()> {
        // No seeding at all: migrations have run, every table is empty. This is
        // the state every freshly provisioned environment starts in.
        let response = catalog(State(state_for(pool)), Query(CatalogQuery::default()))
            .await
            .expect("an unseeded catalog must not be an error");

        let body = serde_json::to_value(&response.0).unwrap();
        assert_eq!(body["updateTime"], "0000-00-00");
        assert_eq!(body["songs"].as_array().unwrap().len(), 0);
        assert_eq!(body["categories"].as_array().unwrap().len(), 0);

        Ok(())
    }
```

If the existing tests in that file call the handler through an assembled `Router` with `tower::ServiceExt::oneshot` instead of calling `catalog(...)` directly, use that style and assert `response.status() == StatusCode::OK` plus the same two body fields. The point of the test is the status and the sentinel, not the calling convention.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p server catalog_on_an_unseeded`
Expected: FAIL — the handler returns `AppError::Database` from `RowNotFound`, so the `.expect(...)` panics (or the status is 500 in the router style).

- [ ] **Step 3: Write the implementation**

In `apps/server/src/queries/catalog.rs`, replace `fetch_update_time`:

```rust
/// `catalog_meta` is a singleton written only by the catalog sync, so it is
/// empty between `migrate` and the first sync — the state every new environment
/// starts in. Report the same `0000-00-00` sentinel the frontend already uses for
/// an empty catalog (`apps/host/src/utils/data.ts:19`) rather than erroring: an
/// unseeded catalog is empty, not broken.
pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    let update_time = sqlx::query_scalar!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_optional(pool)
    .await?;

    Ok(update_time.unwrap_or_else(|| "0000-00-00".to_string()))
}
```

- [ ] **Step 4: Regenerate the offline query cache**

The macro's SQL string is unchanged, but its call shape changed from `fetch_one` to `fetch_optional`. Verify the cache still matches, and regenerate if not:

```sh
set -a; . apps/server/.env; set +a
cargo sqlx prepare --check --workspace -- -p server --all-targets \
  || cargo sqlx prepare --workspace -- -p server --all-targets
```

- [ ] **Step 5: Update the contract**

In `docs/reference/api-contract.md`, change line 59 to record the unseeded value:

```jsonc
  "updateTime": "YYYY-MM-DD"   // bumped whenever any canonical row changes;
                               // "0000-00-00" before the first catalog sync
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p server`
Expected: PASS, including the new test and all pre-existing ones.

Run: `cargo clippy -p server --all-targets -- -D warnings && cargo fmt --check`
Expected: both exit 0.

- [ ] **Step 7: Commit**

```bash
git add apps/server/src/queries/catalog.rs apps/server/src/routes/catalog.rs docs/reference/api-contract.md .sqlx
git commit -m "fix(server): serve an empty catalog on an unseeded database

fetch_one on the catalog_meta singleton made GET /catalog return 500 for
every freshly migrated environment, including production right now.
fetch_optional with the 0000-00-00 sentinel the frontend already uses."
```
