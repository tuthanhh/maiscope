# 02 — `catalog_sync` mints ids for new songs

**What to build:** a song arriving from upstream for the first time gets a
a minted UUID `id`; a song already present keeps the one it has.

The upsert is `ON CONFLICT (song_id) DO UPDATE`, so the insert and update paths
are one statement. The update path must not touch `id` — overwriting it
would change the URL of a song every time any of its fields changed, which is
the precise failure this feature exists to prevent.

**Blocked by:** 01 — the key must already be a UUID.

**Status:** done (2026-11-17)

- [x] New songs are minted an id during the sync
- [x] The `DO UPDATE` branch does not list `id` among its assignments
- [x] A test that runs the sync twice over the same upstream payload and asserts
      every song `id` is unchanged — the regression that would silently break
      every link in the system
- [x] A test that a changed field on an existing song leaves its `id`
      alone
- [x] Tombstoned songs keep their `id`, so a manual rename fix has
      something to re-point

## Comments

**2026-11-17: done, with no change to what the sync does.** Ticket 01 put the
mint in the column default (`uuidv7()`), and the songs upsert never named
`id`, so a new song already gets one and an existing song keeps its own.
This ticket adds the regression tests and a comment on the upsert saying
why `id` must stay out of it. Tests: an unchanged re-sync keeps every id; an
edited song keeps its id; a tombstoned song keeps its id and returns with it.

Checked by mutation: making the update re-mint `id` fails the edited-song
test. The other two still pass under that mutation, since the sync skips
an unchanged row entirely; they guard a re-insert by any other path.
