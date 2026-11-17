# 02 — POST /community/songs

**What to build:** the upload endpoint. Body is a `maidata.txt`; optional
`?officialPublicId=` links it to a catalog song. Publishes immediately.

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

- [ ] `POST /api/community/songs`, `user+`, body `text/plain`
- [ ] Rejects the whole file if any difficulty fails to parse — a partly-stored
      song would leave the library holding charts that cannot render
- [ ] The error names the difficulty *and* the token: which `inote_N`, which
      comma index, what was there
- [ ] **Every rejection logs at `warn`** with the failing token and the
      difficulty. Without it the feedback loop is half-built: the error reaches
      the contributor and nothing reaches the maintainer, because a rejected
      upload writes no row and `community_reports` only targets songs that exist
- [ ] `?officialPublicId=` validated against `songs.id` (the UUID primary
      key, not `song_id`); absent means standalone
- [ ] Note counts computed from the upload parse, not a second pass
- [ ] `DefaultBodyLimit::max(512 * 1024)` on this route, enforced before
      parsing. Chosen from the corpus — p99 38 KB, observed max 353 KB — rather
      than inheriting Axum's 2 MB default. Parsing a 2 MB body is real CPU on a
      shared instance
- [ ] Rate limited by the existing global limiter. No per-user quota: deferred
      knowingly, see `production-v2`'s *Accepted, not solved*
- [ ] Tests: happy path with a real multi-difficulty file, a malformed
      difficulty, an unknown `officialPublicId`, an oversized body,
      unauthenticated

> Our parser gaps become contributor-facing errors — see ADR-0013's
> consequences. [`parser-defects` 03](../../parser-defects/issues/03-utage-tap-modifiers.md)
> (UTAGE `$`/`@`) is a soft blocker: a legitimate chart is refused until it
> ships.

## Comments

**2026-09-16 — the rejection rate was measured, and the gate ships anyway.**

Across the 1928 local `maidata.txt` files, **56 use the tap-star `$`** that
`parser-defects` 03 does not accept — and **19 of those are ordinary non-UTAGE
charts**, so that defect is not a UTAGE nicety but a rejection rate on normal
content. It is a *floor*, not the rate: it counts one known defect, and nobody
has run `parse_chart` over the whole corpus (`crates/simai/tests/corpus.rs` sweeps
nine hand-written fixtures plus a gitignored `local/` that takes chart bodies,
not `maidata.txt`).

Gating this ticket on a measured pass rate — say ≥99% of the corpus parsing
before the endpoint is built — was put as an option and **rejected**. The
endpoint ships as designed and defects are fixed as they are reported.

That choice is what makes the `warn` log above load-bearing rather than
decorative. It is the only channel by which a rejection reaches anyone who can
fix the parser. A table storing rejected uploads (with their text, as ready-made
regression fixtures) was also considered and rejected: it stores content we
declined to host, and its only reader would be the maintainer, occasionally.
Fly's log retention is the accepted limit — a rejection nobody looks at inside
the window is gone.


**2026-11-17: two optional override fields.** The upload page shows the
title and artist parsed from the file as editable fields, and sends them as
overrides only when the uploader changed them: `title` and `artist`,
both optional, both falling back to the parsed values. Source:
`frontend-redesign` ticket 06, upload screen in
[`design-direction.md`](../../../reference/design-direction.md).
