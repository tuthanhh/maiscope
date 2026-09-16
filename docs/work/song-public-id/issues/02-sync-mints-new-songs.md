# 02 — `catalog_sync` mints ids for new songs

**What to build:** a song arriving from upstream for the first time gets a
a minted UUID `id`; a song already present keeps the one it has.

The upsert is `ON CONFLICT (song_id) DO UPDATE`, so the insert and update paths
are one statement. The update path must not touch `id` — overwriting it
would change the URL of a song every time any of its fields changed, which is
the precise failure this feature exists to prevent.

**Blocked by:** 01 — the key must already be a UUID.

**Status:** todo

- [ ] New songs are minted an id during the sync
- [ ] The `DO UPDATE` branch does not list `id` among its assignments
- [ ] A test that runs the sync twice over the same upstream payload and asserts
      every song `id` is unchanged — the regression that would silently break
      every link in the system
- [ ] A test that a changed field on an existing song leaves its `id`
      alone
- [ ] Tombstoned songs keep their `id`, so a manual rename fix has
      something to re-point
