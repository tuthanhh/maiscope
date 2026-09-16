# 04 — GET /community/charts/{id}/chart

**What to build:** the raw simai body for the engine, mirroring the official
chart endpoint's conventions exactly.

`api-contract.md` §2 establishes those for `GET /sheets/{songId}/chart`:
`text/plain`, not a JSON envelope, because the engine wants the text itself and
wrapping it costs the client a parse and an unwrap. Errors are plain text too —
the documented exception to the §6 JSON error shape.

Match both. A second chart endpoint that disagrees with the first about its
content type would be a trap for `useEngine.ts`, which handles one of them today.

**Blocked by:** 01 — tables migration.

**Status:** todo

- [ ] `200` with the simai body as `text/plain`
- [ ] `404` for unknown id or a chart whose song is `removed`, plain-text body
- [ ] No `501` equivalent — unlike official charts there is no blob-only case,
      because ticket 02 refuses anything that will not parse
- [ ] Tests: a real chart round-trips byte-identical, unknown id, removed song
