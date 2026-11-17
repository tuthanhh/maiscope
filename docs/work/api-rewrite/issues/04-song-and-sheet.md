# 04 — `GET /api/songs/{id}` and `GET /api/songs/{id}/sheets/{type}/{difficulty}`

**What to build:** the addressed reads. One song with its sheets serves the
song modal opened by URL; one sheet with its song serves the visualizer's
metadata card. Neither needs the snapshot.

**Blocked by:** 03 (same song and sheet shapes).

**Status:** todo

- [ ] `GET /api/songs/{id}` → one song, shaped exactly as an element of the
      Catalog's `songs[]`
- [ ] `GET /api/songs/{id}/sheets/{type}/{difficulty}` → the sheet's fields
      plus `"song": { …the song's fields, without "sheets" }` — the explicit
      `sheet.song` reference the client builds, not v1's flattening
- [ ] `{difficulty}` is `difficulties.code`; axum percent-decodes the segment,
      so `/sheets/utage/%E5%8D%94` finds `協`. ASCII UTAGE codes (`X`, `J`, `r`)
      are case-sensitive
- [ ] `{id}` not a UUID → `400 bad_request` in the contract's JSON shape, not
      axum's plain-text path rejection
- [ ] Unknown song, or a known song without that sheet → `404 not_found`
- [ ] Tests: each route's shape against seeded rows, a UTAGE code through the
      encoded path, a malformed id, and both 404s
- [ ] Contract: both routes
