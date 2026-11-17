# 03 — `GET /api/catalog`

**What to build:** the snapshot, reshaped for v2. Still nested
`songs[].sheets[]` on the wire (the client builds its flat list with an
explicit `sheet.song`; see `screen-data-requirements.md`), still ETag'd, but
carrying ids, codes and read fields only.

**Blocked by:** 02 (shares the reference queries and types).

**Status:** done

- [x] Shape:

      { "songs": [{ "id", "category", "title", "artist", "bpm", "imageName",
                    "version", "releaseDate", "isLocked", "comment",
                    "sheets": [{ "type", "difficulty", "level", "levelValue",
                                 "internalLevel", "internalLevelValue",
                                 "noteDesigner", "noteCounts", "regions",
                                 "regionOverrides", "hasChart" }] }],
        ...the five reference arrays, as GET /api/reference }

      `id` is the song's UUID, hyphenated. `difficulty` is `difficulties.code`.
      No `songId`, `isNew`, `isSpecial`, `updateTime`, no derived field
      ([ADR-0026](../../../adr/0026-v2-api-carries-codes-and-read-fields-only.md))
- [x] No `?region=` parameter: nothing calls it; region filtering is the
      client's, from `regions` and `regionOverrides`
- [x] Songs in `source_index` order, sheets in theirs, as v1
- [x] ETag / `304` / `Cache-Control` exactly as v1's Catalog (`Freshness`)
- [x] Test: a field-by-field mapping check against seeded rows — every key
      present is listed in the shape above, every listed key maps to its column.
      **Not** a byte comparison with `tests/snapshots/catalog.json`: the
      spec's tripwire treats byte-identity with v1 as a warning sign
- [x] Test: the forbidden keys (derived fields, `songId`, the dropped fields)
      appear nowhere, at any depth — v1's `walk_keys` check, ported
- [x] Contract: v2 Catalog section

## Comments

**2026-11-17 — built.** `queries::official` has two queries with optional
filters — songs by id, sheets by song and by type plus difficulty code — so
issue 04's single song and single sheet reuse them rather than growing their
own SQL. The sheet query joins `difficulties` and emits `code`. Against the
dev database the v2 Catalog matches v1's counts exactly (1879 songs, 7481
sheets, 7065 with a chart), carries `協`-style codes on UTAGE sheets and no
`songId`, and is 2.99 MB against v1's 3.13 MB.
