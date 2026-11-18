# 04 — `GET /api/songs/{id}` and `GET /api/songs/{id}/sheets/{type}/{difficulty}`

**What to build:** the addressed reads. One song with its sheets serves the
song modal opened by URL; one sheet with its song serves the visualizer's
metadata card. Neither needs the snapshot.

**Blocked by:** 03 (same song and sheet shapes).

**Status:** done

- [x] `GET /api/songs/{id}` → one song, shaped exactly as an element of the
      Catalog's `songs[]`
- [x] `GET /api/songs/{id}/sheets/{type}/{difficulty}` → the sheet's fields
      plus `"song": { …the song's fields, without "sheets" }` — the explicit
      `sheet.song` reference the client builds, not v1's flattening
- [x] `{difficulty}` is `difficulties.code`; axum percent-decodes the segment,
      so `/sheets/utage/%E5%8D%94` finds `協`. ASCII UTAGE codes (`X`, `J`, `r`)
      are case-sensitive
- [x] `{id}` not a UUID → `400 bad_request` in the contract's JSON shape, not
      axum's plain-text path rejection
- [x] Unknown song, or a known song without that sheet → `404 not_found`. That
      covers an unknown `{type}` (`std`, `dx`, `utage` exist) and an unknown
      difficulty code: both are validated by the lookup against `sheets` and
      `difficulties.code`, which is the spec's "checked against
      `difficulties`" — no closed enum in code to drift from the table
- [x] Tests: each route's shape against seeded rows, a UTAGE code through the
      encoded path, a malformed id, and both 404s
- [x] Contract: both routes

## Comments

**2026-11-17 — built.** Both routes reuse issue 03's two queries with their
filters set; no new SQL. The id is parsed by hand so a bad one is the JSON
`400`. Besides the handler tests, one test sends `/utage/%E5%8D%94` through
the router to prove axum decodes the segment, and the bracketed name and a
lower-cased ASCII code both miss. Live against the dev database, an encoded
UTAGE path returns its sheet with `"difficulty": "協"`.

**2026-11-17 — review.** Applied: paths go through `routes::extract::ApiPath`,
so even a segment that is not UTF-8 is the JSON `400`; only the canonical
lowercase hyphenated UUID is a song id, so a song has one URL; quoted input
is capped at 64 characters; both reads share a snapshot and the unreachable
"sheet without its song" is an `Internal` error; `/api` has a JSON 404
fallback. No dev sheet has a NULL type or difficulty, so every sheet is
addressable.
