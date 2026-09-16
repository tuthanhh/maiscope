# 03 — GET /community/songs and /community/songs/{id}

**What to build:** browse and detail for the community library.

Deliberately **not** a second copy of `/sheets/search`. That endpoint is 766
lines of filter-building over official columns — category, version, region,
region overrides — and community rows have none of them. Start with title,
artist and "fan charts of this official song", and add filters when something
actually needs them.

**Blocked by:** 01 — tables migration.

**Status:** todo

- [ ] `GET /community/songs?title=&artist=&officialSongId=&page=&pageSize=`
- [ ] Paginated with a `total` field in the envelope, matching the convention in
      `api-contract.md` §6 — not an `X-Total-Count` header
- [ ] `GET /community/songs/{id}` returns song metadata plus its charts
      (difficulty, level, designer, note counts) but **not** chart content —
      that is ticket 04, so a browse page does not ship simai bodies it will
      not render
- [ ] `status = 'removed'` rows are excluded from both, and the detail route
      404s rather than 410s — a takedown should not confirm what was there
- [ ] Tests: pagination boundaries, the officialSongId filter, removed rows
      absent from both
