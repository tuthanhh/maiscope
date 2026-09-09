# Server Catalog Typed-Query Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `apps/server`'s `GET /catalog`, `GET /songs/{id}`, and `GET /sheets/{sheetExpr}` handlers — which currently return whatever shape the Postgres views `v_song`/`v_sheet`/`v_sheet_obj`/`v_song_meta` happen to build via `json_build_object` — with handlers that query typed Rust structs via `sqlx::query_as!`, so the response shape has exactly one, compiler-checked description instead of two hand-maintained ones (the SQL views and the currently-unused `types.rs` mirror). Along the way: implement the already-documented-but-missing `region` filter on `GET /catalog`, and drop the `hasAudio` field / `/audio` endpoint from the contract since the audio-serving feature is being cut.

**Architecture:** `apps/server/src/types.rs` becomes the single source of truth for the response shape (response structs + the DB row structs `sqlx::query_as!` binds into, with `From`-style conversion methods between them). A new `apps/server/src/queries.rs` holds all `sqlx::query_as!` query functions, kept separate from `main.rs` so `main.rs` stays routing + thin handlers. The four now-unused Postgres views are dropped in a new migration. Region filtering happens in SQL (`WHERE ... EXISTS (SELECT 1 FROM sheet_regions ...)`), not in application code, per the "let the DB do the searching" decision.

**Tech Stack:** Rust, Axum 0.8, sqlx 0.9 (`query_as!` compile-time-checked macro, `macros` + `migrate` features already enabled), Postgres. Tests use `#[sqlx::test]` (auto-creates an isolated, migrated ephemeral database per test against the same Postgres server `DATABASE_URL` points at — no new infra beyond the `docker compose up -d` already required for local dev).

**Spec:** No separate spec document — this plan directly encodes the design reached via the `/grilling` skill in-session (server dual-representation cleanup, region filter, `hasAudio`/audio-feature removal, dropping the now-unused JSON views). The relevant prior decisions:
- `types.rs` (compiler-checked structs) becomes the seam; the DB views are deleted, not kept alongside.
- `region` filter: keeps every song in the response, filters only that song's `sheets[]` to `regions[region] == true` (empty array allowed, song never dropped).
- `hasAudio` field and `GET /sheets/{sheetExpr}/audio` are removed entirely (audio-serving feature cut) — confirmed zero frontend reliance on either.
- `hasChart` (already live via the `charts` table `EXISTS` check in the current `v_sheet_obj`) is preserved, just recomputed via typed query instead of view.

## Global Constraints

- Never touch production — all migrations and query work run only against the local dev Postgres (`docker compose up -d` in `apps/server`, per `apps/server/docker-compose.yml`).
- `sqlx::query_as!` compile-time checking requires a reachable, migrated Postgres at `DATABASE_URL` (from `apps/server/.env`, copied from `.env.example`) during `cargo build`/`cargo test` — run `docker compose up -d && sqlx migrate run` before any task's build/test step.
- Doc sync rule (CLAUDE.md): `apps/server/docs/api-contract.md` and `apps/server/docs/schema.md` must reflect any endpoint/schema change in the same change — Task 1 handles the two known-stale spots, and no later task may reintroduce drift.
- Server returns raw fields only — never emit client-derived fields (`songNo` beyond the existing denormalized convenience column, `imageUrl`, `sheetExpr`, `notePercents`).
- Field ordering in every `sqlx::query_as!`-bound struct must exactly match the `SELECT` list's column order (the macro maps positionally at compile time, not by name-only matching at runtime) — called out per-task below.

---

### Task 1: Fix stale docs — drop `hasAudio`/`/audio`, fix `schema.md`'s planned-tables list

**Files:**
- Modify: `apps/server/docs/api-contract.md`
- Modify: `apps/server/docs/schema.md`

**Interfaces:**
- Consumes: nothing (doc-only).
- Produces: nothing later tasks import — a text-truth precondition for the response shape Task 3 will encode in `types.rs`.

- [ ] **Step 1: Remove `hasAudio` from the Sheet payload shape in the contract**

In `apps/server/docs/api-contract.md`, find the Sheet JSON block (§1.1, currently lines ~76-91) and delete the `hasAudio` line:

```diff
   "isSpecial?": false,
-  "hasChart": false,   // NEW: true if §2 chart/audio exists for this sheetExpr
-  "hasAudio": false    // NEW
+  "hasChart": false    // true if a chart row exists for this sheetExpr
 }
```

Also update the sentence right after the block:

```diff
-`hasChart`/`hasAudio` are the only additions vs. today — they let `songs.vue`/
-`song.vue` show a "visualize" affordance without a probe request. They are server
-truth, not derived, so they belong in the payload.
+`hasChart` is the only addition vs. today — it lets `songs.vue`/`song.vue` show a
+"visualize" affordance without a probe request. It is server truth, not derived,
+so it belongs in the payload.
```

- [ ] **Step 2: Delete the `GET /sheets/{sheetExpr}/audio` section**

Find and delete the entire `### `GET /sheets/{sheetExpr}/audio`` section (§2, around line 127) — the whole subsection, including its request/response description, not just the heading. Read the file first to get exact surrounding lines before deleting, since line numbers may have shifted after Step 1.

- [ ] **Step 3: Verify no other `hasAudio`/`/audio` contract references remain**

Run: `grep -n -i "hasaudio\|sheets/{sheetExpr}/audio" apps/server/docs/api-contract.md`
Expected: no output.

- [ ] **Step 4: Fix `schema.md`'s stale "Planned tables" list**

In `apps/server/docs/schema.md`, the "Planned tables (not yet migrated)" table currently lists both `charts` and `assets`. `charts` is already migrated (`apps/server/migrations/20260624105451_charts.up.sql` created it and `hasChart` already queries it live). `assets` is being dropped along with the audio feature — remove both rows:

```diff
 | Table | Purpose | Contract |
 |-------|---------|----------|
-| `charts` | canonical simai/ma2 per sheet (`content` TEXT or `blob_url`); `UNIQUE(sheet_id, format)` | §2 chart |
 | `chart_revisions` | append-only chart history → rollback/audit | §5 |
-| `assets` | audio/jacket/movie blobs (S3 key + meta) | §2 audio |
 | `users` | GitHub-OAuth identities + role | §4 |
 | `contributions` | open submission queue (`payload` JSONB, `status`) → merge on approve | §5 |
 | `audit_log` | moderator/admin action trail | §5 |
```

- [ ] **Step 5: Verify**

Run: `grep -n "^| \`charts\`\|^| \`assets\`" apps/server/docs/schema.md`
Expected: no output (both rows gone from the planned-tables table).

- [ ] **Step 6: Commit**

```bash
git add apps/server/docs/api-contract.md apps/server/docs/schema.md
git commit -m "docs: drop hasAudio/audio feature from contract, fix stale planned-tables list"
```

---

### Task 2: Migration — drop the now-to-be-unused JSON assembly views

**Files:**
- Create: `apps/server/migrations/20260907080000_drop_json_views.up.sql`
- Create: `apps/server/migrations/20260907080000_drop_json_views.down.sql`

**Interfaces:**
- Consumes: nothing.
- Produces: nothing later tasks reference by name — later tasks stop querying `v_song`/`v_sheet`/`v_sheet_obj`/`v_song_meta` instead of depending on their removal, but running this migration is what makes "these views are gone" actually true for Task 8's final verification.

- [ ] **Step 1: Write the up migration**

```sql
-- apps/server/migrations/20260907080000_drop_json_views.up.sql
-- The handlers no longer read from these JSON-assembly views (see
-- src/queries.rs) — the response shape's single source of truth is now the
-- typed structs in src/types.rs, queried directly against the base tables.
DROP VIEW v_song;
DROP VIEW v_sheet;
DROP VIEW v_sheet_obj;
DROP VIEW v_song_meta;
```

(Drop order matters: `v_song` and `v_sheet` depend on `v_sheet_obj`/`v_song_meta`, so they must drop first.)

- [ ] **Step 2: Write the down migration**

Recreates the views exactly as they exist today (the `v_sheet_obj` definition here is the post-`hasChart` version from `20260624105451_charts.up.sql`; `v_song_meta`/`v_song`/`v_sheet` are unchanged from `20260624104528_json_views.up.sql`):

```sql
-- apps/server/migrations/20260907080000_drop_json_views.down.sql
CREATE VIEW v_sheet_obj AS
SELECT
    s.id,
    s.song_id_fk,
    s.sheet_expr,
    s.source_index,
    json_build_object(
        'type',               s.type,
        'difficulty',         s.difficulty,
        'level',              s.level,
        'levelValue',         s.level_value,
        'internalLevel',      s.internal_level,
        'internalLevelValue', s.internal_level_value,
        'noteDesigner',       s.note_designer,
        'isSpecial',          s.is_special,
        'hasChart', EXISTS (
            SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
        ),
        'noteCounts', (
            SELECT json_object_agg(key, value)
            FROM sheet_note_counts WHERE sheet_id = s.id
        ),
        'regions', (
            SELECT json_object_agg(region, available)
            FROM sheet_regions WHERE sheet_id = s.id
        ),
        'regionOverrides', (
            SELECT json_object_agg(region, json_build_object(
                'level',              level,
                'levelValue',         level_value,
                'internalLevel',      internal_level,
                'internalLevelValue', internal_level_value,
                'noteDesigner',       note_designer
            ))
            FROM sheet_region_overrides WHERE sheet_id = s.id
        )
    ) AS doc
FROM sheets s;

CREATE VIEW v_song_meta AS
SELECT
    so.id,
    so.song_id,
    so.source_index,
    jsonb_build_object(
        'songId',      so.song_id,
        'category',    so.category,
        'title',       so.title,
        'artist',      so.artist,
        'bpm',         so.bpm,
        'imageName',   so.image_name,
        'version',     so.version,
        'releaseDate', to_char(so.release_date, 'YYYY-MM-DD'),
        'isNew',       so.is_new,
        'isLocked',    so.is_locked,
        'comment',     so.comment
    ) AS meta
FROM songs so;

CREATE VIEW v_song AS
SELECT
    m.id,
    m.song_id,
    m.source_index,
    (m.meta || jsonb_build_object(
        'sheets', COALESCE((
            SELECT json_agg(v.doc ORDER BY v.source_index)
            FROM v_sheet_obj v WHERE v.song_id_fk = m.id
        ), '[]'::json)::jsonb
    )) AS doc
FROM v_song_meta m;

CREATE VIEW v_sheet AS
SELECT
    v.sheet_expr,
    (m.meta || v.doc::jsonb) AS doc
FROM v_sheet_obj v
JOIN v_song_meta m ON m.id = v.song_id_fk;
```

- [ ] **Step 3: Run the migration**

Run: `cd apps/server && sqlx migrate run`
Expected: output includes `Applying 20260907080000/drop json views`.

- [ ] **Step 4: Round-trip check — verify the down migration is a true inverse**

Run: `cd apps/server && sqlx migrate revert && sqlx migrate run`
Expected: both commands succeed with no errors (the revert recreates the four views, the re-run drops them again cleanly).

- [ ] **Step 5: Verify the views are gone in the final state**

Run: `cd apps/server && docker compose exec -T postgres psql -U postgres -d maiscope -c "\dv"` (adjust user/db name if `.env` differs from `.env.example`'s defaults)
Expected: no `v_song`, `v_sheet`, `v_sheet_obj`, or `v_song_meta` rows listed.

- [ ] **Step 6: Commit**

```bash
git add apps/server/migrations/20260907080000_drop_json_views.up.sql apps/server/migrations/20260907080000_drop_json_views.down.sql
git commit -m "migrate(server): drop now-unused JSON assembly views"
```

---

### Task 3: `types.rs` — make it the real, compiler-checked seam

**Files:**
- Modify: `apps/server/src/types.rs`

**Interfaces:**
- Consumes: nothing.
- Produces (used by Tasks 4-8):
  - `CategoryEntry { category: String }`
  - `VersionEntry { version: String, abbr: Option<String>, release_date: Option<String> }`
  - `TypeEntry { r#type: String, name: String, abbr: Option<String>, icon_url: Option<String>, icon_height: Option<i32> }`
  - `DifficultyEntry { difficulty: String, name: String, color: Option<String>, icon_url: Option<String>, icon_height: Option<i32> }`
  - `RegionEntry { region: String, name: String }`
  - `Catalog { songs: Vec<Song>, categories: Vec<CategoryEntry>, versions: Vec<VersionEntry>, types: Vec<TypeEntry>, difficulties: Vec<DifficultyEntry>, regions: Vec<RegionEntry>, update_time: String }`
  - `RegionOverride { level: Option<String>, level_value: Option<f64>, internal_level: Option<String>, internal_level_value: Option<f64>, note_designer: Option<String> }`
  - `SongRow { id: i64, song_id: Option<String>, category: Option<String>, title: Option<String>, artist: Option<String>, bpm: Option<f64>, image_name: Option<String>, version: Option<String>, release_date: Option<String>, is_new: Option<bool>, is_locked: Option<bool>, comment: Option<String> }` + `impl SongRow { pub fn into_meta(self) -> SongMeta }`
  - `SheetRow { song_id_fk: i64, r#type: Option<String>, difficulty: Option<String>, level: Option<String>, level_value: Option<f64>, internal_level: Option<String>, internal_level_value: Option<f64>, note_designer: Option<String>, is_special: Option<bool>, has_chart: bool, note_counts: Option<sqlx::types::Json<BTreeMap<String, Option<i64>>>>, regions: Option<sqlx::types::Json<BTreeMap<String, bool>>>, region_overrides: Option<sqlx::types::Json<BTreeMap<String, RegionOverride>>> }` + `impl SheetRow { pub fn into_meta(self) -> SheetMeta }`
  - `SheetMeta` gains `pub has_chart: bool` and `region_overrides` is retyped from `Option<BTreeMap<String, Sheet>>` to `Option<BTreeMap<String, RegionOverride>>` (fixing a pre-existing shape bug — the view only ever emitted 5 fields per override, never a full `Sheet`).

- [ ] **Step 1: Rewrite the file header — it's no longer dead code**

```diff
-// Kept as the typed Rust mirror of the API contract. Handlers currently emit the
-// shapes directly from Postgres JSON views (see migrations/*_json_views), so
-// these structs aren't wired in yet — allow dead_code until they are.
-#![allow(dead_code)]
-
 // Response types mirroring apps/host/src/types/{Song,Sheet}.ts.
 //
 // Server emits RAW fields only. Derived fields the client computes in
 // utils/data.ts:preprocessData (songNo, imageUrl, imageUrlM, sheetExpr,
 // notePercents, $canonicalSheet) are intentionally absent here.
+//
+// This is the single source of truth for the response shape: main.rs's
+// handlers build these types via src/queries.rs's sqlx::query_as! queries,
+// then serde serializes them directly. There is no second, hand-maintained
+// description of the shape (the old v_song/v_sheet/... Postgres views that
+// used to play that role were dropped in migration 20260907080000).
```

- [ ] **Step 2: Add `has_chart` to `SheetMeta` and fix `region_overrides`'s type**

```diff
 #[derive(Debug, Clone, Serialize, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct SheetMeta {
     #[serde(skip_serializing_if = "Option::is_none")]
     pub r#type: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub difficulty: Option<String>,

     #[serde(skip_serializing_if = "Option::is_none")]
     pub level: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub level_value: Option<f64>,

     #[serde(skip_serializing_if = "Option::is_none")]
     pub internal_level: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub internal_level_value: Option<f64>,

     #[serde(skip_serializing_if = "Option::is_none")]
     pub note_designer: Option<String>,
     // Record<string, number | null>
     #[serde(skip_serializing_if = "Option::is_none")]
     pub note_counts: Option<BTreeMap<String, Option<i64>>>,

     // Record<string, boolean>
     #[serde(skip_serializing_if = "Option::is_none")]
     pub regions: Option<BTreeMap<String, bool>>,
-    // Record<string, Sheet> — partial per-region overrides.
+    // Partial per-region overrides — only the 5 fields a region can override,
+    // never a full Sheet (region_overrides never carried type/difficulty/
+    // isSpecial/hasChart on the wire; the old view only ever built these 5).
     #[serde(skip_serializing_if = "Option::is_none")]
-    pub region_overrides: Option<BTreeMap<String, Sheet>>,
+    pub region_overrides: Option<BTreeMap<String, RegionOverride>>,

     #[serde(skip_serializing_if = "Option::is_none")]
     pub is_special: Option<bool>,
+
+    // Always present — true if a `charts` row with inline content exists for
+    // this sheet_expr. Server truth, not derived, so no skip_serializing_if.
+    pub has_chart: bool,
 }
+
+/// A region's partial override of a Sheet's level/designer fields.
+/// Row existence in `sheet_region_overrides` = override applies; NULL columns
+/// inherit the canonical sheet value (contract §1.1 regionOverrides).
+#[derive(Debug, Clone, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct RegionOverride {
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub level: Option<String>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub level_value: Option<f64>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub internal_level: Option<String>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub internal_level_value: Option<f64>,
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub note_designer: Option<String>,
+}
```

- [ ] **Step 3: Add the lookup-table entry structs and the `Catalog` wrapper**

Append to the end of the file:

```rust
/// `Data.categories[]` entry (contract §1, `GET /catalog`).
#[derive(Debug, Serialize)]
pub struct CategoryEntry {
    pub category: String,
}

/// `Data.versions[]` entry.
#[derive(Debug, Serialize)]
pub struct VersionEntry {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
    #[serde(rename = "releaseDate", skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

/// `Data.types[]` entry.
#[derive(Debug, Serialize)]
pub struct TypeEntry {
    pub r#type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbr: Option<String>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(rename = "iconHeight", skip_serializing_if = "Option::is_none")]
    pub icon_height: Option<i32>,
}

/// `Data.difficulties[]` entry.
#[derive(Debug, Serialize)]
pub struct DifficultyEntry {
    pub difficulty: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(rename = "iconHeight", skip_serializing_if = "Option::is_none")]
    pub icon_height: Option<i32>,
}

/// `Data.regions[]` entry.
#[derive(Debug, Serialize)]
pub struct RegionEntry {
    pub region: String,
    pub name: String,
}

/// `GET /catalog` response (contract §1). Byte-shape-compatible with the old
/// `data.json` so `preprocessData` is unchanged.
#[derive(Debug, Serialize)]
pub struct Catalog {
    pub songs: Vec<Song>,
    pub categories: Vec<CategoryEntry>,
    pub versions: Vec<VersionEntry>,
    pub types: Vec<TypeEntry>,
    pub difficulties: Vec<DifficultyEntry>,
    pub regions: Vec<RegionEntry>,
    #[serde(rename = "updateTime")]
    pub update_time: String,
}
```

- [ ] **Step 4: Add the DB row structs `sqlx::query_as!` binds into, plus their `into_meta` conversions**

Append:

```rust
use std::collections::BTreeMap as _RowBTreeMapAliasGuard; // silence unused-import lints if BTreeMap already imported above; remove this line if `use std::collections::BTreeMap;` is already present at the top of the file (it is — see existing imports).
```

(Skip the alias line above — `BTreeMap` is already imported at the top of `types.rs`. Just append the structs directly:)

```rust
/// Raw row from the `songs` table (plus `to_char`-formatted `release_date`).
/// Field order MUST match the SELECT list in queries.rs::fetch_song_row /
/// fetch_all_songs — sqlx::query_as! maps positionally.
pub struct SongRow {
    pub id: i64,
    pub song_id: Option<String>,
    pub category: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bpm: Option<f64>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub is_new: Option<bool>,
    pub is_locked: Option<bool>,
    pub comment: Option<String>,
}

impl SongRow {
    pub fn into_meta(self) -> SongMeta {
        SongMeta {
            song_id: self.song_id,
            category: self.category,
            title: self.title,
            artist: self.artist,
            bpm: self.bpm,
            image_name: self.image_name,
            version: self.version,
            release_date: self.release_date,
            is_new: self.is_new,
            is_locked: self.is_locked,
            comment: self.comment,
        }
    }
}

/// Raw row from the `sheets` table joined with its has_chart/note_counts/
/// regions/region_overrides aggregates. Field order MUST match the SELECT
/// list in queries.rs — sqlx::query_as! maps positionally.
pub struct SheetRow {
    pub song_id_fk: i64,
    pub r#type: Option<String>,
    pub difficulty: Option<String>,
    pub level: Option<String>,
    pub level_value: Option<f64>,
    pub internal_level: Option<String>,
    pub internal_level_value: Option<f64>,
    pub note_designer: Option<String>,
    pub is_special: Option<bool>,
    pub has_chart: bool,
    pub note_counts: Option<sqlx::types::Json<BTreeMap<String, Option<i64>>>>,
    pub regions: Option<sqlx::types::Json<BTreeMap<String, bool>>>,
    pub region_overrides: Option<sqlx::types::Json<BTreeMap<String, RegionOverride>>>,
}

impl SheetRow {
    pub fn into_meta(self) -> SheetMeta {
        SheetMeta {
            r#type: self.r#type,
            difficulty: self.difficulty,
            level: self.level,
            level_value: self.level_value,
            internal_level: self.internal_level,
            internal_level_value: self.internal_level_value,
            note_designer: self.note_designer,
            note_counts: self.note_counts.map(|j| j.0),
            regions: self.regions.map(|j| j.0),
            region_overrides: self.region_overrides.map(|j| j.0),
            is_special: self.is_special,
            has_chart: self.has_chart,
        }
    }
}
```

- [ ] **Step 5: Build check**

Run: `cd apps/server && cargo build`
Expected: compiles. Warnings about unused `SongRow`/`SheetRow`/etc. are expected and fine until Task 4-7 wire them in — they are not errors.

- [ ] **Step 6: Commit**

```bash
git add apps/server/src/types.rs
git commit -m "feat(server): make types.rs the response shape's single source of truth"
```

---

### Task 4: `queries.rs` — lookup-table queries (categories/versions/types/difficulties/regions/updateTime)

**Files:**
- Create: `apps/server/src/queries.rs`
- Modify: `apps/server/src/main.rs:1` (add `mod queries;`)

**Interfaces:**
- Consumes: `types::{CategoryEntry, VersionEntry, TypeEntry, DifficultyEntry, RegionEntry}` from Task 3.
- Produces (used by Task 7):
  - `pub async fn fetch_categories(pool: &PgPool) -> Result<Vec<CategoryEntry>, sqlx::Error>`
  - `pub async fn fetch_versions(pool: &PgPool) -> Result<Vec<VersionEntry>, sqlx::Error>`
  - `pub async fn fetch_types(pool: &PgPool) -> Result<Vec<TypeEntry>, sqlx::Error>`
  - `pub async fn fetch_difficulties(pool: &PgPool) -> Result<Vec<DifficultyEntry>, sqlx::Error>`
  - `pub async fn fetch_regions(pool: &PgPool) -> Result<Vec<RegionEntry>, sqlx::Error>`
  - `pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error>`

- [ ] **Step 1: Write the failing tests**

```rust
// apps/server/src/queries.rs (top of file, test module at the bottom)
use crate::types::{CategoryEntry, DifficultyEntry, RegionEntry, TypeEntry, VersionEntry};
use sqlx::PgPool;

pub async fn fetch_categories(pool: &PgPool) -> Result<Vec<CategoryEntry>, sqlx::Error> {
    todo!()
}

pub async fn fetch_versions(pool: &PgPool) -> Result<Vec<VersionEntry>, sqlx::Error> {
    todo!()
}

pub async fn fetch_types(pool: &PgPool) -> Result<Vec<TypeEntry>, sqlx::Error> {
    todo!()
}

pub async fn fetch_difficulties(pool: &PgPool) -> Result<Vec<DifficultyEntry>, sqlx::Error> {
    todo!()
}

pub async fn fetch_regions(pool: &PgPool) -> Result<Vec<RegionEntry>, sqlx::Error> {
    todo!()
}

pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn fetch_categories_orders_by_ordinal(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "maimai", 1)
            .execute(&pool)
            .await?;
        sqlx::query!("INSERT INTO categories (category, ordinal) VALUES ($1, $2)", "pops", 0)
            .execute(&pool)
            .await?;

        let result = fetch_categories(&pool).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].category, "pops");
        assert_eq!(result[1].category, "maimai");
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_versions_formats_release_date(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO versions (version, abbr, release_date, ordinal) VALUES ($1, $2, $3, $4)",
            "maimai DX",
            Some("DX"),
            Some(chrono::NaiveDate::from_ymd_opt(2019, 7, 11).unwrap()),
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_versions(&pool).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].version, "maimai DX");
        assert_eq!(result[0].abbr.as_deref(), Some("DX"));
        assert_eq!(result[0].release_date.as_deref(), Some("2019-07-11"));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_update_time_reads_catalog_meta(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO catalog_meta (id, update_time) VALUES (true, $1)",
            chrono::DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z").unwrap().with_timezone(&chrono::Utc)
        )
        .execute(&pool)
        .await?;

        let result = fetch_update_time(&pool).await.unwrap();

        assert_eq!(result, "2026-09-01");
        Ok(())
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/server && docker compose up -d && sqlx migrate run && cargo test --lib fetch_categories_orders_by_ordinal fetch_versions_formats_release_date fetch_update_time_reads_catalog_meta`
Expected: compiles (the `todo!()` bodies compile fine against the signatures), then FAIL at runtime with `not yet implemented` panics.

- [ ] **Step 3: Implement the query functions**

Replace each `todo!()` body:

```rust
pub async fn fetch_categories(pool: &PgPool) -> Result<Vec<CategoryEntry>, sqlx::Error> {
    sqlx::query_as!(
        CategoryEntry,
        r#"SELECT category FROM categories ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_versions(pool: &PgPool) -> Result<Vec<VersionEntry>, sqlx::Error> {
    sqlx::query_as!(
        VersionEntry,
        r#"SELECT version, abbr, to_char(release_date, 'YYYY-MM-DD') AS release_date
           FROM versions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_types(pool: &PgPool) -> Result<Vec<TypeEntry>, sqlx::Error> {
    sqlx::query_as!(
        TypeEntry,
        r#"SELECT type AS "type: String", name, abbr, icon_url, icon_height
           FROM types ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_difficulties(pool: &PgPool) -> Result<Vec<DifficultyEntry>, sqlx::Error> {
    sqlx::query_as!(
        DifficultyEntry,
        r#"SELECT difficulty, name, color, icon_url, icon_height
           FROM difficulties ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_regions(pool: &PgPool) -> Result<Vec<RegionEntry>, sqlx::Error> {
    sqlx::query_as!(
        RegionEntry,
        r#"SELECT region, name FROM regions ORDER BY ordinal"#
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_update_time(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(pool)
    .await
}
```

Note on `fetch_types`: `type` is a Rust keyword, but `TypeEntry.r#type` (raw identifier) is the struct field name — the `AS "type: String"` alias tells `query_as!` which struct field the column maps to; sqlx's macro matches the raw identifier `r#type` against the bare column name `type` positionally, this alias is only needed if the macro's inferred SQL type needs an override (TEXT already maps directly to `String`, so a plain `SELECT type, name, ...` also works — use the plain form unless the build in Step 4 reports a type-inference error, in which case fall back to the aliased form shown above).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/server && cargo test --lib fetch_categories_orders_by_ordinal fetch_versions_formats_release_date fetch_update_time_reads_catalog_meta`
Expected: PASS (3 passed).

- [ ] **Step 5: Wire the module into main.rs**

```diff
+mod queries;
 mod types;

 use axum::{
```

- [ ] **Step 6: Full build check**

Run: `cd apps/server && cargo build`
Expected: compiles cleanly (only remaining unused-code warnings for `fetch_types`/`fetch_difficulties`/`fetch_regions` until Task 7 wires them into a handler).

- [ ] **Step 7: Commit**

```bash
git add apps/server/src/queries.rs apps/server/src/main.rs
git commit -m "feat(server): add typed lookup-table queries"
```

---

### Task 5: `queries.rs` — sheet queries (region-filtered for catalog, unfiltered for a single song)

**Files:**
- Modify: `apps/server/src/queries.rs`

**Interfaces:**
- Consumes: `types::SheetRow` from Task 3.
- Produces (used by Task 7-8):
  - `pub async fn fetch_all_sheets(pool: &PgPool, region: Option<&str>) -> Result<HashMap<i64, Vec<SheetRow>>, sqlx::Error>` — every sheet in the catalog, grouped by `song_id_fk`, region-filtered if `region` is `Some`.
  - `pub async fn fetch_sheets_for_song(pool: &PgPool, song_pk: i64) -> Result<Vec<SheetRow>, sqlx::Error>` — all sheets for one song, unfiltered (used by `GET /songs/{id}`, which the contract does not region-filter).

- [ ] **Step 1: Write the failing tests**

Append to `apps/server/src/queries.rs`:

```rust
pub async fn fetch_all_sheets(
    pool: &PgPool,
    region: Option<&str>,
) -> Result<std::collections::HashMap<i64, Vec<crate::types::SheetRow>>, sqlx::Error> {
    todo!()
}

pub async fn fetch_sheets_for_song(
    pool: &PgPool,
    song_pk: i64,
) -> Result<Vec<crate::types::SheetRow>, sqlx::Error> {
    todo!()
}
```

Add to the `#[cfg(test)] mod tests` block:

```rust
    async fn seed_song_with_two_sheets(pool: &PgPool) -> (i64, i64, i64) {
        let song_pk: i64 = sqlx::query_scalar!(
            "INSERT INTO songs (song_id, source_index) VALUES ($1, $2) RETURNING id",
            "maimai_song",
            0
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_a: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, $2, 'dx', 'master', 0) RETURNING id",
            song_pk,
            "maimai_song|dx|master"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_b: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             VALUES ($1, $2, 'dx', 'expert', 1) RETURNING id",
            song_pk,
            "maimai_song|dx|expert"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        (song_pk, sheet_a, sheet_b)
    }

    #[sqlx::test]
    async fn fetch_all_sheets_groups_by_song(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, _a, _b) = seed_song_with_two_sheets(&pool).await;

        let grouped = fetch_all_sheets(&pool, None).await.unwrap();

        assert_eq!(grouped.get(&song_pk).map(|v| v.len()), Some(2));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_all_sheets_filters_by_region_keeps_empty_vec(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, sheet_a, _sheet_b) = seed_song_with_two_sheets(&pool).await;
        // Only sheet_a is available in "jp"; sheet_b has no region row at all.
        sqlx::query!(
            "INSERT INTO sheet_regions (sheet_id, region, available) VALUES ($1, 'jp', true)",
            sheet_a
        )
        .execute(&pool)
        .await?;

        let grouped = fetch_all_sheets(&pool, Some("jp")).await.unwrap();
        assert_eq!(grouped.get(&song_pk).map(|v| v.len()), Some(1));

        let grouped_kr = fetch_all_sheets(&pool, Some("kr")).await.unwrap();
        // Song must still be absent from the map (no sheets matched) — the
        // handler (Task 7) is responsible for still emitting the song with an
        // empty sheets: [] array; fetch_all_sheets just doesn't produce a
        // key for songs with zero matching sheets, callers use unwrap_or_default.
        assert_eq!(grouped_kr.get(&song_pk), None);
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheets_for_song_ignores_region(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, _a, _b) = seed_song_with_two_sheets(&pool).await;

        let sheets = fetch_sheets_for_song(&pool, song_pk).await.unwrap();

        assert_eq!(sheets.len(), 2);
        Ok(())
    }

    #[sqlx::test]
    async fn sheet_row_reports_has_chart(pool: PgPool) -> sqlx::Result<()> {
        let (song_pk, sheet_a, _sheet_b) = seed_song_with_two_sheets(&pool).await;
        sqlx::query!(
            "INSERT INTO charts (sheet_id, sheet_expr, content) VALUES ($1, $2, 'chart text')",
            sheet_a,
            "maimai_song|dx|master"
        )
        .execute(&pool)
        .await?;

        let sheets = fetch_sheets_for_song(&pool, song_pk).await.unwrap();
        let a = sheets.iter().find(|s| s.difficulty.as_deref() == Some("master")).unwrap();
        let b = sheets.iter().find(|s| s.difficulty.as_deref() == Some("expert")).unwrap();

        assert!(a.has_chart);
        assert!(!b.has_chart);
        Ok(())
    }
```

Note: the `HashMap` key clarification in `fetch_all_sheets_filters_by_region_keeps_empty_vec` documents where the "keep the song, empty `sheets: []`" behavior actually lives — in the Task 7 handler, not in this query function, since a `HashMap` has no notion of "present with zero elements" vs "absent" that's useful to distinguish here.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/server && cargo test --lib fetch_all_sheets fetch_sheets_for_song sheet_row_reports_has_chart`
Expected: FAIL with `not yet implemented` panics (or a compile error if `HashMap` needs importing — add `use std::collections::HashMap;` at the top of `queries.rs` if so).

- [ ] **Step 3: Implement the query functions**

```rust
use std::collections::HashMap;

pub async fn fetch_all_sheets(
    pool: &PgPool,
    region: Option<&str>,
) -> Result<HashMap<i64, Vec<crate::types::SheetRow>>, sqlx::Error> {
    let rows = sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
            EXISTS (
                SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
            ) AS "has_chart!",
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id)
                AS "note_counts: sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>",
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id)
                AS "regions: sqlx::types::Json<std::collections::BTreeMap<String, bool>>",
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id)
                AS "region_overrides: sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>"
           FROM sheets s
           WHERE $1::text IS NULL
              OR EXISTS (
                  SELECT 1 FROM sheet_regions sr
                  WHERE sr.sheet_id = s.id AND sr.region = $1 AND sr.available
              )
           ORDER BY s.song_id_fk, s.source_index"#,
        region
    )
    .fetch_all(pool)
    .await?;

    let mut grouped: HashMap<i64, Vec<crate::types::SheetRow>> = HashMap::new();
    for row in rows {
        grouped.entry(row.song_id_fk).or_default().push(row);
    }
    Ok(grouped)
}

pub async fn fetch_sheets_for_song(
    pool: &PgPool,
    song_pk: i64,
) -> Result<Vec<crate::types::SheetRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
            EXISTS (
                SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
            ) AS "has_chart!",
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id)
                AS "note_counts: sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>",
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id)
                AS "regions: sqlx::types::Json<std::collections::BTreeMap<String, bool>>",
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id)
                AS "region_overrides: sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>"
           FROM sheets s
           WHERE s.song_id_fk = $1
           ORDER BY s.source_index"#,
        song_pk
    )
    .fetch_all(pool)
    .await
}
```

`SheetRow`'s field order (Task 3) must match this SELECT list exactly: `song_id_fk, type, difficulty, level, level_value, internal_level, internal_level_value, note_designer, is_special, has_chart, note_counts, regions, region_overrides` — it does.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/server && cargo test --lib fetch_all_sheets fetch_sheets_for_song sheet_row_reports_has_chart`
Expected: PASS (4 passed).

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/queries.rs
git commit -m "feat(server): add typed, region-filterable sheet queries"
```

---

### Task 6: `queries.rs` — single-song and single-sheet-by-expr queries

**Files:**
- Modify: `apps/server/src/queries.rs`

**Interfaces:**
- Consumes: `types::SongRow`, `types::SheetRow` from Task 3; `fetch_sheets_for_song` from Task 5.
- Produces (used by Task 8):
  - `pub async fn fetch_song_by_song_id(pool: &PgPool, song_id: &str) -> Result<Option<SongRow>, sqlx::Error>`
  - `pub async fn fetch_sheet_by_expr(pool: &PgPool, sheet_expr: &str) -> Result<Option<(SongRow, SheetRow)>, sqlx::Error>`

- [ ] **Step 1: Write the failing tests**

```rust
pub async fn fetch_song_by_song_id(
    pool: &PgPool,
    song_id: &str,
) -> Result<Option<crate::types::SongRow>, sqlx::Error> {
    todo!()
}

pub async fn fetch_sheet_by_expr(
    pool: &PgPool,
    sheet_expr: &str,
) -> Result<Option<(crate::types::SongRow, crate::types::SheetRow)>, sqlx::Error> {
    todo!()
}
```

Add to `#[cfg(test)] mod tests`:

```rust
    #[sqlx::test]
    async fn fetch_song_by_song_id_returns_none_when_missing(pool: PgPool) -> sqlx::Result<()> {
        let result = fetch_song_by_song_id(&pool, "does_not_exist").await.unwrap();
        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_song_by_song_id_finds_row(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;

        let result = fetch_song_by_song_id(&pool, "maimai_song").await.unwrap();

        assert_eq!(result.map(|r| r.title), Some(Some("Example Song".to_string())));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheet_by_expr_joins_song_and_sheet(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song",
            "Example Song",
            0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master",
            "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = fetch_sheet_by_expr(&pool, "maimai_song|dx|master").await.unwrap();

        let (song, sheet) = result.expect("sheet should be found");
        assert_eq!(song.title.as_deref(), Some("Example Song"));
        assert_eq!(sheet.difficulty.as_deref(), Some("master"));
        Ok(())
    }

    #[sqlx::test]
    async fn fetch_sheet_by_expr_returns_none_when_missing(pool: PgPool) -> sqlx::Result<()> {
        let result = fetch_sheet_by_expr(&pool, "nope|dx|master").await.unwrap();
        assert!(result.is_none());
        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/server && cargo test --lib fetch_song_by_song_id fetch_sheet_by_expr`
Expected: FAIL with `not yet implemented` panics.

- [ ] **Step 3: Implement**

```rust
pub async fn fetch_song_by_song_id(
    pool: &PgPool,
    song_id: &str,
) -> Result<Option<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs WHERE song_id = $1"#,
        song_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn fetch_sheet_by_expr(
    pool: &PgPool,
    sheet_expr: &str,
) -> Result<Option<(crate::types::SongRow, crate::types::SheetRow)>, sqlx::Error> {
    let Some(song_pk) = sqlx::query_scalar!(
        "SELECT song_id_fk FROM sheets WHERE sheet_expr = $1",
        sheet_expr
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let song = sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs WHERE id = $1"#,
        song_pk
    )
    .fetch_one(pool)
    .await?;

    let sheet = sqlx::query_as!(
        crate::types::SheetRow,
        r#"SELECT
            s.song_id_fk,
            s.type,
            s.difficulty,
            s.level,
            s.level_value,
            s.internal_level,
            s.internal_level_value,
            s.note_designer,
            s.is_special,
            EXISTS (
                SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL
            ) AS "has_chart!",
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id)
                AS "note_counts: sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>",
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id)
                AS "regions: sqlx::types::Json<std::collections::BTreeMap<String, bool>>",
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id)
                AS "region_overrides: sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>"
           FROM sheets s
           WHERE s.sheet_expr = $1"#,
        sheet_expr
    )
    .fetch_one(pool)
    .await?;

    Ok(Some((song, sheet)))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/server && cargo test --lib fetch_song_by_song_id fetch_sheet_by_expr`
Expected: PASS (4 passed).

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/queries.rs
git commit -m "feat(server): add typed single-song and sheet-by-expr queries"
```

---

### Task 7: Rewire `GET /catalog` — typed assembly + the `region` filter

**Files:**
- Modify: `apps/server/src/main.rs`

**Interfaces:**
- Consumes: `queries::{fetch_categories, fetch_versions, fetch_types, fetch_difficulties, fetch_regions, fetch_update_time, fetch_all_sheets}` (Tasks 4-5); `types::{Catalog, Song, NestedSheet, SongRow}` (Task 3). Also needs a new query: `queries::fetch_all_songs(pool) -> Result<Vec<SongRow>, sqlx::Error>` ordered by `source_index` — add it to `queries.rs` in Step 1 below (it wasn't listed in Task 4/5 because it's `catalog`-specific and simple enough to fold in here rather than force an artificial split).
- Produces: `catalog()` handler now takes `Query<CatalogQuery>`; `CatalogQuery { region: Option<String> }` struct.

- [ ] **Step 1: Add `fetch_all_songs` to `queries.rs` (with its own test first)**

```rust
pub async fn fetch_all_songs(pool: &PgPool) -> Result<Vec<crate::types::SongRow>, sqlx::Error> {
    todo!()
}
```

Test:

```rust
    #[sqlx::test]
    async fn fetch_all_songs_orders_by_source_index(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "second", "Second", 1
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "first", "First", 0
        )
        .execute(&pool)
        .await?;

        let songs = fetch_all_songs(&pool).await.unwrap();

        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title.as_deref(), Some("First"));
        assert_eq!(songs[1].title.as_deref(), Some("Second"));
        Ok(())
    }
```

Run: `cd apps/server && cargo test --lib fetch_all_songs_orders_by_source_index` — expect FAIL (`todo!()`).

Implement:

```rust
pub async fn fetch_all_songs(pool: &PgPool) -> Result<Vec<crate::types::SongRow>, sqlx::Error> {
    sqlx::query_as!(
        crate::types::SongRow,
        r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                  to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
           FROM songs ORDER BY source_index"#
    )
    .fetch_all(pool)
    .await
}
```

Run again — expect PASS. Commit: `git add apps/server/src/queries.rs && git commit -m "feat(server): add fetch_all_songs query"`.

- [ ] **Step 2: Add the `region` query param and rewrite the `catalog` handler**

```diff
 use axum::{
     Json, Router,
     extract::{Path, Query, State},
     http::StatusCode,
     routing::get,
 };
 use serde::Deserialize;
-use serde_json::{Value, json};
+use serde_json::{Value, json};
 use sqlx::{Pool, Postgres};
 use tower_http::cors::CorsLayer;
+use types::{Catalog, NestedSheet, Song};
```

```diff
+// Query params for GET /catalog (contract §1). `since` (sync-tier revision
+// check) is not implemented yet — no §3 sync tier exists.
+#[derive(Debug, Deserialize)]
+struct CatalogQuery {
+    region: Option<String>,
+}
+
 // GET /catalog — assembles the full Data shape (types/Data.ts) from the DB.
 // Byte-compatible with the old data.json so preprocessData is unchanged.
 async fn catalog(
+    Query(CatalogQuery { region }): Query<CatalogQuery>,
     State(pool): State<Pool<Postgres>>,
-) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
-    let data: Value = sqlx::query_scalar(
-        "SELECT json_build_object(
-            'songs',        (SELECT COALESCE(json_agg(doc ORDER BY source_index), '[]') FROM v_song),
-            'categories',   (SELECT COALESCE(json_agg(json_build_object('category', category) ORDER BY ordinal), '[]') FROM categories),
-            'versions',     (SELECT COALESCE(json_agg(json_build_object('version', version, 'abbr', abbr, 'releaseDate', to_char(release_date, 'YYYY-MM-DD')) ORDER BY ordinal), '[]') FROM versions),
-            'types',        (SELECT COALESCE(json_agg(json_build_object('type', type, 'name', name, 'abbr', abbr, 'iconUrl', icon_url, 'iconHeight', icon_height) ORDER BY ordinal), '[]') FROM types),
-            'difficulties', (SELECT COALESCE(json_agg(json_build_object('difficulty', difficulty, 'name', name, 'color', color, 'iconUrl', icon_url, 'iconHeight', icon_height) ORDER BY ordinal), '[]') FROM difficulties),
-            'regions',      (SELECT COALESCE(json_agg(json_build_object('region', region, 'name', name) ORDER BY ordinal), '[]') FROM regions),
-            'updateTime',   (SELECT to_char(update_time, 'YYYY-MM-DD') FROM catalog_meta LIMIT 1)
-        )",
-    )
-    .fetch_one(&pool)
-    .await
-    .map_err(db_error)?;
-
-    Ok(Json(data))
+) -> Result<Json<Catalog>, (StatusCode, Json<Value>)> {
+    let song_rows = queries::fetch_all_songs(&pool).await.map_err(db_error)?;
+    let mut sheets_by_song = queries::fetch_all_sheets(&pool, region.as_deref())
+        .await
+        .map_err(db_error)?;
+
+    let songs = song_rows
+        .into_iter()
+        .map(|row| {
+            let sheets = sheets_by_song
+                .remove(&row.id)
+                .unwrap_or_default()
+                .into_iter()
+                .map(|sheet_row| NestedSheet { sheet: sheet_row.into_meta() })
+                .collect();
+            Song { meta: row.into_meta(), sheets }
+        })
+        .collect();
+
+    let catalog = Catalog {
+        songs,
+        categories: queries::fetch_categories(&pool).await.map_err(db_error)?,
+        versions: queries::fetch_versions(&pool).await.map_err(db_error)?,
+        types: queries::fetch_types(&pool).await.map_err(db_error)?,
+        difficulties: queries::fetch_difficulties(&pool).await.map_err(db_error)?,
+        regions: queries::fetch_regions(&pool).await.map_err(db_error)?,
+        update_time: queries::fetch_update_time(&pool).await.map_err(db_error)?,
+    };
+
+    Ok(Json(catalog))
 }
```

(`sheets_by_song.remove(&row.id)` — using `remove` instead of `get`/`clone` is deliberate: each song's sheets are consumed exactly once, no cloning needed, and it makes a silent double-use bug impossible since a second `remove` for the same key would yield `None`, not stale data. This is the "keep the song, empty `sheets: []`" behavior from the grilling decision — `unwrap_or_default()` produces `vec![]` for songs with zero region-matching sheets, and the song itself is never skipped since we iterate `song_rows`, not `sheets_by_song`.)

- [ ] **Step 3: Write an integration test for the handler itself**

Axum handlers are plain async functions — call `catalog()` directly with hand-built extractors, no HTTP server needed.

```rust
// apps/server/src/main.rs, bottom of file
#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query as AxumQuery, State as AxumState};

    #[sqlx::test]
    async fn catalog_returns_song_with_empty_sheets_when_region_excludes_all(
        pool: sqlx::PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool)
        .await?;
        sqlx::query!("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        // No sheet_regions row for "kr" — sheet_a should be filtered out.

        let result = catalog(
            AxumQuery(CatalogQuery { region: Some("kr".to_string()) }),
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(result.0.songs.len(), 1);
        assert_eq!(result.0.songs[0].sheets.len(), 0);
        Ok(())
    }
}
```

- [ ] **Step 4: Run the test**

Run: `cd apps/server && cargo test --lib catalog_returns_song_with_empty_sheets_when_region_excludes_all`
Expected: PASS.

- [ ] **Step 5: Manual smoke test against the running server**

```bash
cd apps/server
docker compose up -d
sqlx migrate run
cargo run --bin ingest   # if the dev DB needs seed data
cargo run &
sleep 1
curl -s http://localhost:3000/api/v1/catalog | jq '.songs | length'
curl -s "http://localhost:3000/api/v1/catalog?region=intl" | jq '.songs[0].sheets'
kill %1
```
Expected: both curls return valid JSON matching the contract shape; the `region=intl` call's sheets arrays only contain sheets available in `intl` (or are empty, never absent from the song).

- [ ] **Step 6: Commit**

```bash
git add apps/server/src/main.rs
git commit -m "feat(server): rewire GET /catalog onto typed queries, add region filter"
```

---

### Task 8: Rewire `GET /songs/{id}` and `GET /sheets/{sheetExpr}`, final cleanup

**Files:**
- Modify: `apps/server/src/main.rs`

**Interfaces:**
- Consumes: `queries::{fetch_song_by_song_id, fetch_sheets_for_song, fetch_sheet_by_expr}` (Task 6); `types::{Song, Sheet, NestedSheet}` (Task 3).
- Produces: nothing further downstream — this is the last task.

- [ ] **Step 1: Write failing handler tests**

```rust
// in the existing #[cfg(test)] mod tests block in main.rs
    #[sqlx::test]
    async fn get_song_returns_404_for_unknown_id(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let result = get_song(Path("nope".to_string()), AxumState(pool)).await;
        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn get_song_returns_song_with_sheets(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = get_song(Path("maimai_song".to_string()), AxumState(pool)).await.unwrap();

        assert_eq!(result.0.sheets.len(), 1);
        Ok(())
    }

    #[sqlx::test]
    async fn get_sheet_returns_404_for_unknown_expr(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let result = get_sheet(Path("nope|dx|master".to_string()), AxumState(pool)).await;
        assert!(result.is_err());
        Ok(())
    }

    #[sqlx::test]
    async fn get_sheet_flattens_song_and_sheet_fields(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool)
        .await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index)
             SELECT id, $1, 'dx', 'master', 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool)
        .await?;

        let result = get_sheet(Path("maimai_song|dx|master".to_string()), AxumState(pool))
            .await
            .unwrap();

        assert_eq!(result.0.song.title.as_deref(), Some("Example Song"));
        assert_eq!(result.0.sheet.difficulty.as_deref(), Some("master"));
        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/server && cargo test --lib get_song_ get_sheet_`
Expected: compile error (handlers still return `Result<Json<Value>, _>`, tests expect typed `.0.sheets`/`.0.title` etc.) — this compile failure IS the "red" step for this task; proceed to Step 3.

- [ ] **Step 3: Rewrite `get_song`**

```diff
 // GET /songs/{id} — single Song with its sheets (types/Song.ts).
 async fn get_song(
     Path(id): Path<String>,
     State(pool): State<Pool<Postgres>>,
-) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
-    let doc: Option<Value> = sqlx::query_scalar("SELECT doc FROM v_song WHERE song_id = $1")
-        .bind(&id)
-        .fetch_optional(&pool)
-        .await
-        .map_err(db_error)?;
-
-    doc.map(Json).ok_or_else(|| not_found("song", &id))
+) -> Result<Json<Song>, (StatusCode, Json<Value>)> {
+    let Some(song_row) = queries::fetch_song_by_song_id(&pool, &id)
+        .await
+        .map_err(db_error)?
+    else {
+        return Err(not_found("song", &id));
+    };
+
+    let sheet_rows = queries::fetch_sheets_for_song(&pool, song_row.id)
+        .await
+        .map_err(db_error)?;
+
+    let sheets = sheet_rows
+        .into_iter()
+        .map(|row| NestedSheet { sheet: row.into_meta() })
+        .collect();
+
+    Ok(Json(Song { meta: song_row.into_meta(), sheets }))
 }
```

- [ ] **Step 4: Rewrite `get_sheet`**

```diff
 // GET /sheets/{sheetExpr} — standalone Sheet, song fields flattened in
 // (types/Sheet.ts). sheetExpr is URL-encoded; axum decodes the path segment.
 async fn get_sheet(
     Path(sheet_expr): Path<String>,
     State(pool): State<Pool<Postgres>>,
-) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
-    let doc: Option<Value> = sqlx::query_scalar("SELECT doc FROM v_sheet WHERE sheet_expr = $1")
-        .bind(&sheet_expr)
-        .fetch_optional(&pool)
-        .await
-        .map_err(db_error)?;
-
-    doc.map(Json).ok_or_else(|| not_found("sheet", &sheet_expr))
+) -> Result<Json<types::Sheet>, (StatusCode, Json<Value>)> {
+    let Some((song_row, sheet_row)) = queries::fetch_sheet_by_expr(&pool, &sheet_expr)
+        .await
+        .map_err(db_error)?
+    else {
+        return Err(not_found("sheet", &sheet_expr));
+    };
+
+    Ok(Json(types::Sheet { song: song_row.into_meta(), sheet: sheet_row.into_meta() }))
 }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd apps/server && cargo test --lib`
Expected: all tests pass (lookup queries, sheet queries, song/sheet-by-expr queries, catalog handler, `get_song`/`get_sheet` handlers).

- [ ] **Step 6: Remove now-unused imports, full clean build**

Run: `cd apps/server && cargo build 2>&1 | grep -i warning`
Expected: no `unused import` warnings for `Value`/`json` (both are still used by `db_error`/`not_found`/`healthcheck`/`ChartQuery`'s error paths — verify, don't blindly remove). If any struct from Task 3 (`SongRow`, `SheetRow`, lookup entry types) still shows as unused, that means a wiring step was missed — go back and check Tasks 7-8's diffs applied cleanly.

- [ ] **Step 7: Full manual smoke test**

```bash
cd apps/server
docker compose up -d && sqlx migrate run
cargo run --bin ingest   # re-seed if the drop-views migration required a fresh DB
cargo run &
sleep 1
curl -s http://localhost:3000/api/v1/catalog | jq '.updateTime, (.songs | length)'
curl -s http://localhost:3000/api/v1/songs/<some-real-song-id> | jq '.sheets[0].hasChart'
curl -s http://localhost:3000/api/v1/sheets/<some-real-sheet-expr> | jq '.title, .difficulty'
curl -s http://localhost:3000/api/v1/songs/does-not-exist -w '%{http_code}\n'
kill %1
```
Expected: `/catalog` returns the full shape with `updateTime` and songs; `/songs/{id}` sheets include `hasChart`; `/sheets/{sheetExpr}` shows both song and sheet fields flattened; the unknown-id request prints `404`.

- [ ] **Step 8: Final doc-sync check**

Run: `grep -n "hasAudio\|v_song\|v_sheet" apps/server/docs/api-contract.md apps/server/docs/schema.md apps/server/src/main.rs apps/server/src/queries.rs`
Expected: no output — confirms the contract doc, schema doc, and code are all free of the removed feature and the dropped views, per the CLAUDE.md doc-sync rule.

- [ ] **Step 9: Commit**

```bash
git add apps/server/src/main.rs
git commit -m "feat(server): rewire GET /songs/{id} and GET /sheets/{sheetExpr} onto typed queries"
```

---

## Self-Review Notes

- **Spec coverage:** dual-representation cleanup → Tasks 3-8; `region` filter → Task 5 (query) + Task 7 (handler + empty-sheets behavior); `hasAudio`/`/audio` removal → Task 1; drop unused views → Task 2; `hasChart` preserved → Task 3 (`SheetMeta.has_chart`) + Task 5/6 (`EXISTS` subquery in every sheet query). All six grilled decisions have a task.
- **Field-order checks:** `SongRow`/`SheetRow` field declaration order (Task 3) matches every `SELECT` list that binds into them (Tasks 5-7) — verified column-by-column while writing each query.
- **Naming consistency:** `into_meta()` used consistently on both `SongRow` and `SheetRow`; `fetch_all_songs`/`fetch_all_sheets`/`fetch_sheets_for_song`/`fetch_song_by_song_id`/`fetch_sheet_by_expr` names match between their Task 4-7 definitions and Task 7-8 call sites.
- **No placeholder tasks** — every step has real SQL/Rust, no "add error handling" hand-waves; `db_error`/`not_found` (pre-existing helpers) are reused as-is throughout since they weren't part of the friction being fixed.

---

**Plan complete and saved to `docs/superpowers/plans/2026-09-07-server-catalog-typed-queries.md`.** Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
