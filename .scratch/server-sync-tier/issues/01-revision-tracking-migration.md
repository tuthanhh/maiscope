# 01 — revision + tombstone tables migration

**What to build:** A monotonic `revision BIGINT` on `catalog_meta`, plus `revision` columns on `songs`/`sheets`, plus `catalog_meta.last_full_reload_revision` (so `GET /sync/delta` can detect "too old to diff across a full TRUNCATE reload" precisely). `deleted_songs`/`deleted_sheets` tombstone tables created now with a complete shape, populated later once any deletion capability exists (none does yet in this codebase).

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `catalog_meta` gains `revision`, `last_full_reload_revision` (both default 0)
- [ ] `songs`/`sheets` gain `revision` (indexed)
- [ ] `deleted_songs`/`deleted_sheets` tables created, indexed on `revision`
- [ ] `sqlx migrate run && sqlx migrate revert && sqlx migrate run` round-trips cleanly
