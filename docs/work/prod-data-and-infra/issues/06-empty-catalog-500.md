# 06 — `GET /catalog` 500s on an unseeded database

**What to build:** A freshly migrated database has schema but no rows, and
`GET /api/v1/catalog` returns **500** rather than an empty catalog. Found on the
live Fly deploy (2026-09-13), reproduced locally against the same Neon database via
the direct endpoint, so it is not a pooler artifact:

```
ERROR request{method=GET uri=/api/v1/catalog}: database query failed
      error=no rows returned by a query that expected to return at least one row
```

The cause is `apps/server/src/queries/catalog.rs:52`:

```rust
sqlx::query_scalar!(r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#)
    .fetch_one(pool)
```

`catalog_meta` is a singleton written by `bin/sync_catalog`, and no migration seeds
it. So the window between `migrate` and the first sync — which is exactly the state
every new environment starts in, and the state production is in right now — serves
500s on the main read endpoint.

The write path itself is fine: `bin/ingest` no longer exists — it was replaced by
the differential `bin/sync_catalog`, which never truncates (see
`04-seed-workflows.md`) — and `catalog_sync.rs:83-89` writes `catalog_meta` via an
`INSERT ... ON CONFLICT (id) DO UPDATE ... RETURNING revision` upsert, which
returns exactly one row whether or not the singleton already existed. So a
first-ever sync against an empty database works fine. Only the read path is
brittle.

**Not merely cosmetic.** It makes "provision a new environment" a two-step process
where step one leaves the API returning 500, and it means a truncation bug or a
failed reload turns a degraded catalog into a hard outage.

**Blocked by:** None

**Status:** done — a4fd0b5

- [x] `fetch_update_time` uses `fetch_optional`, or the query is restructured so an
      empty `catalog_meta` is representable (`apps/server/src/queries/catalog.rs:61`,
      falls back to the `0000-00-00` sentinel)
- [x] **Contract decision**: what does the response carry when no catalog exists?
      Options are `updateTime: null`, omitting the field, or 503 "not yet seeded".
      Whichever wins goes in `docs/reference/api-contract.md` in the same change
      (decided: `"0000-00-00"` before the first catalog sync — documented at
      `docs/reference/api-contract.md:59-60` and `:212`)
- [x] The frontend tolerates that shape — `utils/data.ts:preprocessData` currently
      assumes the field is present (`buildEmptyData()` in `apps/host/src/utils/data.ts:19`
      already uses the same `'0000-00-00'` sentinel as its own empty-state marker,
      so the server's value round-trips without special-casing)
- [x] Regression test: `#[sqlx::test]` with no seed data asserting the endpoint does
      not 500 (`apps/server/src/routes/catalog.rs:312`,
      `catalog_on_an_unseeded_database_is_empty_not_an_error`)
- [x] Audit the other read endpoints for the same pattern; `fetch_one` on a table an
      empty database legitimately has zero rows in is the smell (`routes/sync.rs`'s
      `catalog_meta` reads now use `fetch_optional` too, at lines 82 and 128; its
      other `fetch_one` calls are `COUNT(*)` queries, which always return one row
      regardless of table emptiness, so they were never the same bug)

**This ticket was already fixed before this plan was written.** Commit `a4fd0b5`
("fix(server): serve an empty catalog on an unseeded database") landed the
`fetch_optional` change in `queries::fetch_update_time` and, in the same commit,
fixed the identical `fetch_one`-on-an-empty-singleton bug in `Freshness::load`
(`apps/server/src/routes/caching.rs`), which `GET /sync/manifest` hits as well as
`GET /catalog`. A sibling case survived in `GET /sync/delta` (`routes/sync.rs`,
which has its own `catalog_meta` reads and was not in that commit's file list) and
was closed separately by commit `62f27b6` ("fix(server): /sync/delta no longer
500s on an unseeded database").

## Comments

**Contract note.** `updateTime: null` is the smallest change and keeps one code path,
but pushes an "is it seeded?" check onto every client. A 503 is more honest about the
server not being ready and would let Fly's health check catch an unseeded deploy —
at the cost of making `/catalog` fail for a reason clients cannot act on. Worth
deciding deliberately rather than by whichever is easiest to type.
