# 02 — `GET /api/healthcheck` and `GET /api/reference`

**What to build:** the first two v2 read routes. The healthcheck moves to the
v2 root, still exempt from rate limiting. The reference tables are the boot
fetch [ADR-0021](../../../adr/0021-catalog-snapshot-plus-reference-tables-no-server-search.md)
calls for: five small ordered lists, so any screen resolves names without the
3MB Catalog.

**Blocked by:** 01.

**Status:** todo

- [ ] `GET /api/healthcheck` → as v1's, outside every limiter (a router test
      proves the exemption, as v1's does)
- [ ] `GET /api/reference` →

      { "categories":   [{ "category" }],
        "versions":     [{ "version", "abbr" }],
        "types":        [{ "type", "name", "abbr" }],
        "difficulties": [{ "code", "name", "color" }],
        "regions":      [{ "region", "name" }] }

      each in Catalog order. No icon fields, no `versions[].releaseDate`
      ([ADR-0026](../../../adr/0026-v2-api-carries-codes-and-read-fields-only.md))
- [ ] `difficulties` is keyed by `code`, the form sheets and URLs carry; every
      derived UTAGE row is present (`catalog-difficulties`)
- [ ] ETag from `Freshness` and `304` on `If-None-Match`: the tables change with
      the Catalog and ride its revision. `Cache-Control` as the Catalog's
- [ ] The response types are new structs in `types.rs`, not v1's entries with
      fields skipped — v1's must stay byte-identical until 07
- [ ] Contract: a v2 section for both routes
