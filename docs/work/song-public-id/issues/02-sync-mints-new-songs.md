# 02 — `catalog_sync` mints ids for new songs

**What to build:** a song arriving from upstream for the first time gets a
`public_id`; a song already present keeps the one it has.

The upsert is `ON CONFLICT (song_id) DO UPDATE`, so the insert and update paths
are one statement. The update path must not touch `public_id` — overwriting it
would change the URL of a song every time any of its fields changed, which is
the precise failure this feature exists to prevent.

**Blocked by:** 01 — the column must exist.

**Status:** todo

- [ ] New songs are minted an id during the sync
- [ ] The `DO UPDATE` branch does not list `public_id` among its assignments
- [ ] A test that runs the sync twice over the same upstream payload and asserts
      every `public_id` is unchanged — the regression that would silently break
      every link in the system
- [ ] A test that a changed field on an existing song leaves its `public_id`
      alone
- [ ] Tombstoned songs keep their `public_id`, so a manual rename fix has
      something to re-point
