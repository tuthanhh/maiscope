# 03 — types.rs becomes the compiler-checked response-shape seam

**What to build:** `apps/server/src/types.rs` becomes the single source of truth for every response shape this feature touches: response structs (`CategoryEntry`, `VersionEntry`, `TypeEntry`, `DifficultyEntry`, `RegionEntry`, `Catalog`, `RegionOverride`) plus the DB row structs (`SongRow`, `SheetRow`) that `sqlx::query_as!` binds into, with `into_meta()` conversions to the existing `SongMeta`/`SheetMeta`. Also fixes a pre-existing shape bug: `SheetMeta.region_overrides` was typed as `Option<BTreeMap<String, Sheet>>` but only ever carried 5 fields — retyped to `Option<BTreeMap<String, RegionOverride>>`. `SheetMeta` gains `has_chart: bool`.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] `#![allow(dead_code)]` removed from `types.rs` header (no longer dead — this is now the real seam)
- [ ] `SheetMeta` gains `has_chart: bool`; `region_overrides` retyped to `Option<BTreeMap<String, RegionOverride>>`
- [ ] `RegionOverride` struct added (level/levelValue/internalLevel/internalLevelValue/noteDesigner, camelCase)
- [ ] `CategoryEntry`, `VersionEntry`, `TypeEntry`, `DifficultyEntry`, `RegionEntry`, `Catalog` structs added, matching contract shape
- [ ] `SongRow` + `impl SongRow::into_meta() -> SongMeta` added, field order matching future SELECT lists
- [ ] `SheetRow` + `impl SheetRow::into_meta() -> SheetMeta` added, field order matching future SELECT lists
- [ ] `cargo build` compiles clean (unused-code warnings for the new structs are expected until later tickets wire them in)
