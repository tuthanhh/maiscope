# 02 — `GET /api/healthcheck` and `GET /api/reference`

**What to build:** the first two v2 read routes. The healthcheck moves to the
v2 root, still exempt from rate limiting. The reference tables are the boot
fetch [ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md)
calls for: five small ordered lists, so any screen resolves names without the
3MB Catalog.

**Blocked by:** 01.

**Status:** done

- [x] `GET /api/healthcheck` → as v1's, outside every limiter (a router test
      proves the exemption, as v1's does)
- [x] `GET /api/reference` →

      { "categories":   [{ "category" }],
        "versions":     [{ "version", "abbr" }],
        "types":        [{ "type", "name", "abbr" }],
        "difficulties": [{ "code", "name", "color" }],
        "regions":      [{ "region", "name" }] }

      each in Catalog order. No icon fields, no `versions[].releaseDate`
      ([ADR-0026](../../../adr/0026-v2-api-carries-codes-and-read-fields-only.md))
- [x] `difficulties` is keyed by `code`, the form sheets and URLs carry; every
      derived UTAGE row is present (`catalog-difficulties`)
- [x] ETag from `Freshness` and `304` on `If-None-Match`: the tables change with
      the Catalog and ride its revision. `Cache-Control` as the Catalog's
- [x] The response types are new structs in `types.rs`, not v1's entries with
      fields skipped — v1's must stay byte-identical until 07
- [x] Contract: a v2 section for both routes

## Comments

**2026-11-17 — built.** `queries::fetch_reference` returns all five tables in
one call; the Catalog (03) reuses it. The reads sit behind the main limiter
and share its bucket with `/api/v1`, which the router test now shows: once
the bucket is spent, `/api/reference` is throttled while both healthchecks
still answer. The test pins the exact JSON, so a stray icon field or a
`difficulty` key fails it.

**2026-11-17 — review.** Applied: the 304 test checks the repeated `ETag` and
`Cache-Control`, and a new test shows a revision bump moving the ETag and a
stale validator getting the body. The handler comments why it loads the
validator before the body. The review found the reference ETag stale after
`catalog-difficulties`' UTAGE backfill, which added rows without a revision
bump; migration `20261117130000` bumps the revision once at the v2 deploy,
which also stops every client reading its cached v1 Catalog as current.
Intended, not changed: a `304` revalidation spends a rate-limit token, as the
Catalog's always has.
