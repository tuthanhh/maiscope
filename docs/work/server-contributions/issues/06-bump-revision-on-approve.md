# 06 — Bump revision on contribution-approve edit (optional)

**What to build:** `approve_contribution`'s `edit` branch bumps `catalog_meta.revision` atomically (via `UPDATE ... RETURNING revision`) and stamps the new revision onto the affected sheet's `revision` column, replacing the old unconditional `update_time`-only bump — so an incremental edit becomes visible to `GET /sync/delta`. This ticket is optional: if the contributions feature's approve handler doesn't exist yet or looks different, skip it — manifest/delta still work correctly with revision only ever bumped by `ingest`.

**Blocked by:** 03 — approve and reject, `server-sync-tier/issues/01-revision-tracking-migration.md`.

**Status:** todo

- [ ] `edit` branch of `approve_contribution` bumps `catalog_meta.revision` (`+1`, `RETURNING revision`) and `update_time` in one atomic query
- [ ] Same call stamps the new revision onto the edited sheet's `revision` column
- [ ] Old standalone `UPDATE catalog_meta SET update_time = now()` call after the match block removed (folded into the branch-local bump)
- [ ] `cargo build` compiles
