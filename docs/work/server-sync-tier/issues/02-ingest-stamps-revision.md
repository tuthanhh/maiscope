# 02 — ingest.rs stamps revision on full reload

**What to build:** `apps/server/src/bin/ingest.rs` computes a fresh `new_revision` (previous + 1) at the start of every run, stamps it onto `catalog_meta.revision`/`last_full_reload_revision` and every inserted `songs`/`sheets` row — since `ingest` does a full `TRUNCATE...CASCADE` reload, every row from a given run shares the same revision and that revision also marks the "can't diff across this point" boundary.

**Blocked by:** 01 — revision tracking migration.

**Status:** done

- [ ] `new_revision` computed from the pre-truncate `catalog_meta.revision` (+1), inside the same transaction
- [ ] `catalog_meta` insert sets both `revision` and `last_full_reload_revision` to `new_revision`
- [ ] Every `songs`/`sheets` insert stamps `revision = new_revision`
- [ ] `cargo run --bin ingest` then `SELECT revision, last_full_reload_revision FROM catalog_meta` shows both incremented correctly
