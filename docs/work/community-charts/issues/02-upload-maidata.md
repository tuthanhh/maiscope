# 02 — POST /community/songs

**What to build:** the upload endpoint. Body is a `maidata.txt`; optional
`?officialSongId=` links it to a catalog song. Publishes immediately.

Flow: parse the file → validate every `inote_N` with `simai::parse_chart` →
reject the whole upload if any fails → insert one `community_songs` row and one
`community_charts` row per difficulty, with note counts from the parse.

Rejection is the interesting path. `ParseError` already carries the token index,
the token text and the cause, which is what a contributor needs to fix it
themselves. Surface all three; do not flatten to "invalid chart".

**Blocked by:** 01 — tables migration;
`parser-crate-extraction/issues/03-server-dependency-and-ci.md`;
`server-auth-github-oauth/issues/02-jwt-and-authuser-extractor.md`.

**Status:** todo

- [ ] `POST /community/songs`, `user+`, body `text/plain`
- [ ] Rejects the whole file if any difficulty fails to parse — a partly-stored
      song would leave the library holding charts that cannot render
- [ ] The error names the difficulty *and* the token: which `inote_N`, which
      comma index, what was there
- [ ] `?officialSongId=` validated against `songs.id`; absent means standalone
- [ ] Note counts computed from the upload parse, not a second pass
- [ ] A size cap, enforced before parsing
- [ ] Rate limited like every other write route
- [ ] Tests: happy path with a real multi-difficulty file, a malformed
      difficulty, an unknown `officialSongId`, an oversized body, unauthenticated

> Our parser gaps become contributor-facing errors — see ADR-0013's
> consequences. [`parser-defects` 03](../../parser-defects/issues/03-utage-tap-modifiers.md)
> (UTAGE `$`/`@`) is a soft blocker: a legitimate UTAGE chart is refused until
> it ships.
