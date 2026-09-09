# 02 — Migration: drop the now-unused Postgres JSON assembly views

**What to build:** A migration that drops the four Postgres views (`v_song`, `v_sheet`, `v_sheet_obj`, `v_song_meta`) that currently assemble the API response shape via `json_build_object`, once the response shape's single source of truth moves to typed Rust structs. Down migration recreates them exactly as they exist today, verified to round-trip.

**Blocked by:** None — can start immediately (later tickets 07/08 in this feature stop querying these views; this ticket just makes their removal real).

**Status:** done

- [ ] `apps/server/migrations/20260907080000_drop_json_views.up.sql` drops `v_song`, `v_sheet` before `v_sheet_obj`, `v_song_meta` (dependency order)
- [ ] `apps/server/migrations/20260907080000_drop_json_views.down.sql` recreates all four views exactly as today
- [ ] `sqlx migrate run` applies cleanly
- [ ] `sqlx migrate revert && sqlx migrate run` round-trips with no errors
- [ ] `psql \dv` shows none of the four views after the up migration
