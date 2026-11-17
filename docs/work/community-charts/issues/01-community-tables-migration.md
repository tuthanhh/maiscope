# 01 — community_songs, community_charts, community_reports migration

**What to build:** three tables holding the community library. Nothing in
`songs`, `sheets` or `charts` changes — not a column, not an index.

```
community_songs     id                -- uuid v7 PRIMARY KEY, the API's address
                    uploader_user_id  → users (NOT NULL)
                    official_song_id  → songs (NULL = standalone fan song)
                    title, artist, bpm, genre, version   -- from &title= &artist= &wholebpm= &genre= &version=
                    extra_keys JSONB                     -- maidata keys this app does not read
                    status TEXT CHECK (published|removed)
                    created_at, updated_at

community_charts    id                -- uuid v7 PRIMARY KEY, the API's address
                    community_song_id → community_songs ON DELETE CASCADE
                    difficulty        -- inote slot mapped to sheets.difficulty codes
                    level, designer   -- from &lv_N= &des_N=
                    content           -- the simai body
                    note_counts JSONB -- computed from the upload-time parse
                    created_at
                    UNIQUE (community_song_id, difficulty)

community_reports   id
                    community_song_id → community_songs
                    community_chart_id → community_charts (NULL = whole song)
                    reporter_user_id  → users
                    reason, created_at
                    resolved_at, resolved_by, resolution   -- NULL while open
```

`official_song_id` is `ON DELETE SET NULL`, not `CASCADE`: `catalog_sync` never
deletes, but if a song ever does vanish upstream the fan chart of it should
survive as a standalone, not disappear with it.

`note_counts` is stored rather than derived on read because the upload already
parses the chart to validate it — the counts are a by-product, and recomputing
them per request would mean parsing on every page view.

**Blocked by:** `server-auth-github-oauth/issues/01-users-table-migration.md`
(the `users` FKs).

**Status:** todo

- [ ] Migration adds the three tables and nothing else
- [ ] No `pending` value anywhere — ADR-0013 has no review state
- [ ] `status` defaults to `published`
- [ ] `community_songs.id` and `community_charts.id` are `uuid` primary keys,
      minted v7 by the same code as `songs.id`
      ([`song-public-id`](../../song-public-id/spec.md)) — one addressing scheme
      across both libraries, one id per row. No bigserial and no second
      identifier: a sequence in a path would publish the library's size, and
      that cannot be unpublished afterwards
- [ ] `community_reports.id` stays a bigserial — it is never public, so there is
      nothing to leak and nothing to bookmark
- [ ] `difficulty` references `difficulties.code`, not a free string — the bare
      form (`協`), which is what the parser emits
      ([`catalog-difficulties`](../../catalog-difficulties/spec.md) issue 01)
- [ ] Index on `community_songs(official_song_id)` — the community-side
      "fan charts of this song" lookup
- [ ] Index on `community_reports(resolved_at)` filtered to open reports
- [ ] Test: tombstoning an uploader (`queries::tombstone_user`) leaves their
      `community_songs`, `community_charts` and `community_reports` rows
      untouched (carried from `server-auth-github-oauth` 06, which shipped
      before these tables existed)
- [ ] `.sqlx/` regenerated (`cargo sqlx prepare --workspace -- -p server --all-targets`)
- [ ] `docs/reference/schema.md` gains the tables
