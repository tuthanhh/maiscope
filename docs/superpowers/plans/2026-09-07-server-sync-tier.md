# Server Sync Tier (Manifest + Delta) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `apps/server`'s §3 Sync tier — `GET /sync/manifest` (cheap freshness probe) and `GET /sync/delta?since=` (incremental changes) — the backend contract a future local-SQLite offline cache (`src-tauri`) will consume. Per the contract's own framing ("No frontend code yet; this is its contract"), this plan is server-only; the Tauri-side SQLite cache itself is separate future frontend work.

**Architecture:** A monotonic `revision` counter on `catalog_meta`, plus a `revision` column on `songs` and `sheets` (set to the current revision whenever a row is inserted/updated) so `GET /sync/delta?since=X` can select `WHERE revision > X`. A critical constraint this plan works around rather than ignores: `apps/server/src/bin/ingest.rs` does a full `TRUNCATE ... RESTART IDENTITY CASCADE` and reloads from scratch on every run — there is no way to diff *across* a full reload (old and new rows share no stable identity from the diff's perspective), so this plan adds `catalog_meta.last_full_reload_revision` and makes `GET /sync/delta` return the contract's own documented `409 snapshot_required` whenever `since` predates the last full reload, rather than attempting a nonsensical diff. Tombstones get a real table and response shape, but no producer yet — nothing in this codebase can currently delete a canonical `song`/`sheet` row (the contributions plan doesn't add a delete endpoint either, matching the contract, which doesn't define one) — so the `tombstones` field in `GET /sync/delta`'s response will always be empty arrays until a deletion capability exists somewhere. `GET /catalog`'s documented-but-unimplemented `ETag`/`If-None-Match` support (contract §1, line 54-55) is implemented here too, since it shares the same `catalogHash` concept as the manifest.

**Tech Stack:** Reuses `sqlx`/`axum`; `sha2` (already added by the auth plan) for `catalogHash`.

**Spec:** `apps/server/docs/api-contract.md` §3 (lines 139-166) and §1's ETag lines (54-55) are the spec. One real, load-bearing deviation from a literal reading of §3, driven by `ingest.rs`'s actual TRUNCATE-based reload (a fact about this codebase, not a design preference): "if `since` is too old to diff, respond `409`" is interpreted precisely as "since predates the last full reload," not a vague staleness heuristic.

## Global Constraints

- Depends on the contributions plan (`docs/superpowers/plans/2026-09-07-server-contributions.md`) only for Task 3's revision-bump hook into `approve_contribution` — if that plan hasn't landed yet, Task 3's edit can be deferred/adjusted to whatever the current approve handler looks like, or skipped (manifest/delta still work correctly with revision only ever bumped by `ingest`, just with fewer incremental-update opportunities to observe).
- Never touch production.
- Doc sync rule: `api-contract.md` updated in Task 4.
- `revision` is a strictly increasing `BIGINT`, never reused, never decremented — every consumer (client cache) treats gaps as normal (a gap doesn't mean data is missing, just that some revision numbers were consumed by writes that didn't touch the rows the client cares about).

---

### Task 1: `revision` tracking migration

**Files:**
- Create: `apps/server/migrations/20260907110000_sync_revision.up.sql`
- Create: `apps/server/migrations/20260907110000_sync_revision.down.sql`

**Interfaces:**
- Produces: `catalog_meta.revision BIGINT`, `catalog_meta.last_full_reload_revision BIGINT`, `songs.revision BIGINT`, `sheets.revision BIGINT`, `deleted_songs`/`deleted_sheets` tombstone tables.

- [ ] **Step 1: Write the migration**

```sql
-- apps/server/migrations/20260907110000_sync_revision.up.sql
-- Sync tier (contract §3): a monotonic revision counter so clients can ask
-- "what changed since revision N" instead of refetching the whole catalog.

ALTER TABLE catalog_meta
    ADD COLUMN revision BIGINT NOT NULL DEFAULT 0,
    -- The revision at the last full TRUNCATE+reload (see bin/ingest.rs).
    -- GET /sync/delta can only diff incrementally *after* this point — a full
    -- reload has no stable row identity to diff against the client's cache.
    ADD COLUMN last_full_reload_revision BIGINT NOT NULL DEFAULT 0;

ALTER TABLE songs  ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;
ALTER TABLE sheets ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;

CREATE INDEX songs_revision_idx  ON songs  (revision);
CREATE INDEX sheets_revision_idx ON sheets (revision);

-- Deletion log. Nothing in this codebase can delete a canonical song/sheet
-- yet (no such endpoint exists in the contract) — these tables exist so
-- GET /sync/delta's response shape is complete now, populated once a
-- deletion capability is added later.
CREATE TABLE deleted_songs (
    song_id    TEXT   NOT NULL,
    revision   BIGINT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (song_id, revision)
);
CREATE TABLE deleted_sheets (
    sheet_expr TEXT   NOT NULL,
    revision   BIGINT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (sheet_expr, revision)
);

CREATE INDEX deleted_songs_revision_idx ON deleted_songs (revision);
CREATE INDEX deleted_sheets_revision_idx ON deleted_sheets (revision);
```

- [ ] **Step 2: Write the down migration**

```sql
-- apps/server/migrations/20260907110000_sync_revision.down.sql
DROP TABLE IF EXISTS deleted_sheets;
DROP TABLE IF EXISTS deleted_songs;
ALTER TABLE sheets DROP COLUMN revision;
ALTER TABLE songs DROP COLUMN revision;
ALTER TABLE catalog_meta DROP COLUMN last_full_reload_revision;
ALTER TABLE catalog_meta DROP COLUMN revision;
```

- [ ] **Step 3: Run and round-trip check**

Run: `cd apps/server && sqlx migrate run && sqlx migrate revert && sqlx migrate run`
Expected: all succeed.

- [ ] **Step 4: Commit**

```bash
git add apps/server/migrations/20260907110000_sync_revision.up.sql apps/server/migrations/20260907110000_sync_revision.down.sql
git commit -m "feat(server): add revision tracking + tombstone tables for sync tier"
```

---

### Task 2: `ingest.rs` sets the revision on full reload

**Files:**
- Modify: `apps/server/src/bin/ingest.rs`

**Interfaces:** none new — modifies existing INSERT statements to also set `revision`.

- [ ] **Step 1: Compute the next revision at the start of `ingest`**

```diff
 async fn ingest(pool: &PgPool, data: &RawData) -> Result<(), Box<dyn Error>> {
     let mut tx = pool.begin().await?;
+
+    // Every full reload gets a fresh revision, and marks itself as the point
+    // GET /sync/delta can't diff across (see the sync-tier plan). Read the
+    // previous revision from the about-to-be-truncated catalog_meta first.
+    let previous_revision: i64 = sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
+        .fetch_optional(&mut *tx)
+        .await?
+        .unwrap_or(0);
+    let new_revision = previous_revision + 1;

     // Wipe canonical tables; CASCADE clears sheets + sub-tables via FKs.
     sqlx::query(
         "TRUNCATE catalog_meta, categories, versions, types, difficulties, regions, songs \
          RESTART IDENTITY CASCADE",
     )
```

- [ ] **Step 2: Set `revision`/`last_full_reload_revision` on the `catalog_meta` insert**

Find the existing `INSERT INTO catalog_meta (id, update_time) VALUES (true, $1)` (line 184 in the current file) and extend it:

```diff
-    sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, $1)")
+    sqlx::query(
+        "INSERT INTO catalog_meta (id, update_time, revision, last_full_reload_revision) \
+         VALUES (true, $1, $2, $2)",
+    )
         .bind(now)
+        .bind(new_revision)
         .execute(&mut *tx)
         .await?;
```

(`now` here refers to whatever variable the existing code already binds for `update_time` — check the surrounding lines and keep that name; don't introduce a second timestamp variable.)

- [ ] **Step 3: Set `revision` on every song/sheet insert**

```diff
         let song_pk: i64 = sqlx::query_scalar(
             "INSERT INTO songs \
              (song_id, song_no, category, title, artist, bpm, image_name, version, \
-              release_date, is_new, is_locked, comment, source_index) \
-             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
+              release_date, is_new, is_locked, comment, source_index, revision) \
+             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id",
         )
         .bind(&song.song_id)
         .bind(si as i32 + 1)
         .bind(&song.category)
         .bind(&song.title)
         .bind(&song.artist)
         .bind(song.bpm)
         .bind(&song.image_name)
         .bind(&song.version)
         .bind(parse_date(&song.release_date))
         .bind(song.is_new)
         .bind(song.is_locked)
         .bind(&song.comment)
         .bind(si as i32)
+        .bind(new_revision)
         .fetch_one(&mut *tx)
         .await?;
```

```diff
             let sheet_pk: i64 = sqlx::query_scalar(
                 "INSERT INTO sheets \
                  (song_id_fk, sheet_expr, type, difficulty, level, level_value, \
-                  internal_level, internal_level_value, note_designer, is_special, source_index) \
-                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
+                  internal_level, internal_level_value, note_designer, is_special, source_index, revision) \
+                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
             )
             .bind(song_pk)
             .bind(&expr)
             .bind(&sheet.r#type)
             .bind(&sheet.difficulty)
             .bind(&sheet.level)
             .bind(sheet.level_value)
             .bind(&sheet.internal_level)
             .bind(sheet.internal_level_value)
             .bind(&sheet.note_designer)
             .bind(sheet.is_special)
             .bind(shi as i32)
+            .bind(new_revision)
             .fetch_one(&mut *tx)
             .await?;
```

- [ ] **Step 4: Run ingest and verify**

Run: `cd apps/server && cargo run --bin ingest`
Then: `docker compose exec -T postgres psql -U postgres -d maiscope -c "SELECT revision, last_full_reload_revision FROM catalog_meta;"`
Expected: both columns show `1` (or one higher than whatever it was before, if run more than once).

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/bin/ingest.rs
git commit -m "feat(server): stamp revision on every full ingest reload"
```

---

### Task 3: Bump `revision` on contribution approve (optional — see Global Constraints)

**Files:**
- Modify: `apps/server/src/main.rs` (`approve_contribution`, from the contributions plan)

**Interfaces:** none new.

- [ ] **Step 1: Add a revision bump inside `approve_contribution`'s `edit` branch**

Immediately after the `edit` branch's `UPDATE sheets ...` call (and before the existing `UPDATE catalog_meta SET update_time = now()` line), insert:

```diff
+            let new_revision: i64 = sqlx::query_scalar!(
+                "UPDATE catalog_meta SET revision = revision + 1 WHERE id = true RETURNING revision"
+            )
+            .fetch_one(&pool)
+            .await
+            .map_err(db_error)?;
+
+            sqlx::query!(
+                "UPDATE sheets SET revision = $1 WHERE sheet_expr = $2",
+                new_revision,
+                sheet_expr,
+            )
+            .execute(&pool)
+            .await
+            .map_err(db_error)?;
```

This replaces the standalone `UPDATE catalog_meta SET update_time = now() WHERE id = true` call that currently runs unconditionally after the `match` block — fold that into this same query (`revision = revision + 1, update_time = now()`) so both bump atomically together:

```diff
-    // Bump catalog_meta.update_time so GET /catalog's ETag reflects the merge.
-    sqlx::query!("UPDATE catalog_meta SET update_time = now() WHERE id = true")
-        .execute(&pool)
-        .await
-        .map_err(db_error)?;
+    // (revision + update_time are bumped inside the `edit` branch above, for
+    // kinds that actually touch canonical data. `sheet`/`song`/`chart` return
+    // 501 before reaching here, so there's nothing to bump for them yet.)
```

- [ ] **Step 2: Build check**

Run: `cd apps/server && cargo build`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add apps/server/src/main.rs
git commit -m "feat(server): bump revision on contribution-approve edits"
```

---

### Task 4: `GET /sync/manifest`, `GET /sync/delta`, and `GET /catalog`'s ETag

**Files:**
- Modify: `apps/server/src/main.rs`
- Modify: `apps/server/src/types.rs`

**Interfaces:**
- Produces: `GET /sync/manifest`, `GET /sync/delta` routes; `catalog()` handler gains `ETag`/`If-None-Match` handling.

- [ ] **Step 1: A shared `catalog_hash` helper**

```rust
fn catalog_hash(revision: i64, update_time: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{revision}:{update_time}").as_bytes());
    format!("{:x}", hasher.finalize())
}
```

- [ ] **Step 2: `GET /sync/manifest`**

```rust
async fn sync_manifest(
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let row = sqlx::query!(
        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
           FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    let song_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM songs"#)
        .fetch_one(&pool).await.map_err(db_error)?;
    let sheet_count: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM sheets"#)
        .fetch_one(&pool).await.map_err(db_error)?;
    let chart_count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM charts WHERE content IS NOT NULL"#
    )
    .fetch_one(&pool).await.map_err(db_error)?;

    Ok(Json(json!({
        "updateTime": row.update_time,
        "revision": row.revision,
        "catalogHash": catalog_hash(row.revision, &row.update_time),
        "counts": { "songs": song_count, "sheets": sheet_count, "charts": chart_count }
    })))
}
```

- [ ] **Step 3: `GET /sync/delta?since={revision}`**

```rust
#[derive(Debug, Deserialize)]
struct DeltaQuery {
    since: i64,
}

async fn sync_delta(
    Query(q): Query<DeltaQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let last_full_reload: i64 = sqlx::query_scalar!(
        r#"SELECT last_full_reload_revision AS "v!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    if q.since < last_full_reload {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({ "error": "snapshot_required" })),
        ));
    }

    let changed_song_ids: Vec<i64> = sqlx::query_scalar!(
        r#"SELECT id AS "id!" FROM songs WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    let mut songs = Vec::with_capacity(changed_song_ids.len());
    for song_pk in changed_song_ids {
        let song_row = sqlx::query_as!(
            crate::types::SongRow,
            r#"SELECT id, song_id, category, title, artist, bpm, image_name, version,
                      to_char(release_date, 'YYYY-MM-DD') AS release_date, is_new, is_locked, comment
               FROM songs WHERE id = $1"#,
            song_pk
        )
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;
        let sheet_rows = queries::fetch_sheets_for_song(&pool, song_pk).await.map_err(db_error)?;
        let sheets = sheet_rows.into_iter().map(|r| types::NestedSheet { sheet: r.into_meta() }).collect();
        songs.push(types::Song { meta: song_row.into_meta(), sheets });
    }

    // Sheets that changed on their own (e.g. a future targeted sheet edit)
    // but whose parent song didn't otherwise change — avoid double-counting
    // sheets already included via a changed parent song above.
    let already_included_song_pks: std::collections::HashSet<i64> =
        songs.iter().map(|_| 0).collect(); // placeholder set; see note below
    let _ = already_included_song_pks; // see Self-Review: sheet-only-changed delta is a known gap

    let current_revision: i64 = sqlx::query_scalar!(
        r#"SELECT revision AS "v!" FROM catalog_meta LIMIT 1"#
    )
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    let deleted_song_ids: Vec<String> = sqlx::query_scalar!(
        r#"SELECT song_id AS "v!" FROM deleted_songs WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;
    let deleted_sheet_exprs: Vec<String> = sqlx::query_scalar!(
        r#"SELECT sheet_expr AS "v!" FROM deleted_sheets WHERE revision > $1"#,
        q.since
    )
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(json!({
        "revision": current_revision,
        "songs": songs,
        "charts": [], // chart-meta deltas deferred, see Self-Review
        "tombstones": { "songIds": deleted_song_ids, "sheetExprs": deleted_sheet_exprs }
    })))
}
```

The `edit`-kind contribution approve flow (Task 3) only bumps a *sheet's* `revision`, not its parent song's — meaning `sync_delta`'s current `WHERE revision > $1` on `songs` alone would miss a sheet-level-only edit entirely. Fix by also selecting songs whose *sheets* changed:

```diff
-    let changed_song_ids: Vec<i64> = sqlx::query_scalar!(
-        r#"SELECT id AS "id!" FROM songs WHERE revision > $1"#,
-        q.since
-    )
+    let changed_song_ids: Vec<i64> = sqlx::query_scalar!(
+        r#"SELECT DISTINCT so.id AS "id!" FROM songs so
+           LEFT JOIN sheets s ON s.song_id_fk = so.id
+           WHERE so.revision > $1 OR s.revision > $1"#,
+        q.since
+    )
```

Remove the dead `already_included_song_pks` placeholder block above (Step 3's first draft) — it's superseded by this `JOIN`-based fix, which makes "sheet changed but song didn't" naturally included without needing separate de-duplication logic (a song either appears once from the `DISTINCT` or not at all).

```diff
-    // Sheets that changed on their own (e.g. a future targeted sheet edit)
-    // but whose parent song didn't otherwise change — avoid double-counting
-    // sheets already included via a changed parent song above.
-    let already_included_song_pks: std::collections::HashSet<i64> =
-        songs.iter().map(|_| 0).collect(); // placeholder set; see note below
-    let _ = already_included_song_pks; // see Self-Review: sheet-only-changed delta is a known gap
-
     let current_revision: i64 = sqlx::query_scalar!(
```

- [ ] **Step 4: `GET /catalog`'s ETag/If-None-Match**

```diff
 async fn catalog(
     Query(CatalogQuery { region }): Query<CatalogQuery>,
+    headers: axum::http::HeaderMap,
     State(pool): State<Pool<Postgres>>,
-) -> Result<Json<Catalog>, (StatusCode, Json<Value>)> {
+) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
+    let meta = sqlx::query!(
+        r#"SELECT to_char(update_time, 'YYYY-MM-DD') AS "update_time!", revision AS "revision!"
+           FROM catalog_meta LIMIT 1"#
+    )
+    .fetch_one(&pool)
+    .await
+    .map_err(db_error)?;
+    let etag = format!("\"{}\"", catalog_hash(meta.revision, &meta.update_time));
+
+    if headers.get(axum::http::header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
+        return Ok(StatusCode::NOT_MODIFIED.into_response());
+    }
+
     let song_rows = queries::fetch_all_songs(&pool).await.map_err(db_error)?;
     ...
-    Ok(Json(catalog))
+    Ok((
+        [(axum::http::header::ETAG, etag)],
+        Json(catalog),
+    )
+        .into_response())
 }
```

`into_response()` needs `axum::response::IntoResponse` imported (already available via `axum::response::Redirect`'s import path in the auth plan, or add `use axum::response::IntoResponse;` explicitly at the top of `main.rs` if not already present).

- [ ] **Step 5: Wire routes**

```diff
+                .route("/sync/manifest", get(sync_manifest))
+                .route("/sync/delta", get(sync_delta))
```

- [ ] **Step 6: Manual smoke test**

```bash
cd apps/server && cargo run &
sleep 1
curl -s http://localhost:3000/api/v1/sync/manifest | jq
curl -s "http://localhost:3000/api/v1/sync/delta?since=0" | jq '.revision, (.songs | length)'
curl -si "http://localhost:3000/api/v1/sync/delta?since=99999999" | head -3   # expect 409 if that's above last_full_reload_revision
ETAG=$(curl -si http://localhost:3000/api/v1/catalog | grep -i etag | tr -d '\r')
echo "$ETAG"
curl -si http://localhost:3000/api/v1/catalog -H "If-None-Match: ${ETAG#etag: }" | head -3   # expect 304
kill %1
```

- [ ] **Step 7: Commit**

```bash
git add apps/server/src/main.rs apps/server/src/types.rs
git commit -m "feat(server): add GET /sync/manifest, GET /sync/delta, and catalog ETag support"
```

---

### Task 5: Update `api-contract.md`

**Files:**
- Modify: `apps/server/docs/api-contract.md`

- [ ] **Step 1: Note the 409 semantics precisely**

```diff
 If `since` is too old to diff, respond `409` with
 `{ "error": "snapshot_required" }` → client refetches `GET /catalog`.
+
+> Precisely: `since` is "too old" when it predates the revision of the last
+> full `bin/ingest` reload (`ingest` fully truncates and reloads canonical
+> tables, so there is no stable row identity to diff across that boundary).
+> Incremental contribution-approve edits never trigger this — only a full
+> re-ingest does.
```

- [ ] **Step 2: Note the chart-delta scope cut**

```diff
 ### `GET /sync/delta?since={revision}`
 Rows changed since `revision`. `tombstones` carry deletions.
 ```jsonc
 {
   "revision": 0,
   "songs":  [ /* changed Song (with sheets) */ ],
-  "charts": [ /* changed chart meta, see §2 (no chart body) */ ],
+  "charts": [], // always empty for now — chart-meta delta tracking is a follow-up
   "tombstones": { "songIds": ["..."], "sheetExprs": ["..."] }
 }
 ```
```

- [ ] **Step 3: Commit**

```bash
git add apps/server/docs/api-contract.md
git commit -m "docs: clarify sync-tier 409 semantics, note chart-delta scope cut"
```

---

## Self-Review Notes

- **Spec coverage:** `GET /sync/manifest` ✓, `GET /sync/delta` (songs + tombstone shape) ✓, `409 snapshot_required` ✓ (precisely defined against `ingest`'s TRUNCATE behavior, a real fact about this codebase rather than a guess), `GET /catalog` ETag/304 ✓ (was documented but unimplemented before this plan).
- **Explicit scope cuts:** chart-meta deltas (`charts: []` always) — no chart-content-versioning system to diff against yet (the contributions plan's chart-kind merge is itself deferred, so there's nothing to produce chart deltas from); tombstones structurally complete but never populated (no deletion endpoint exists anywhere in this codebase's contract).
- **Dependency note:** Task 3 is marked optional because it modifies a handler from the contributions plan — if that plan's `approve_contribution` doesn't exist yet or looks different, Tasks 1-2 and 4-5 still work standalone (revision only advances via `ingest`).
- **Type consistency:** `sync_delta` reuses `types::SongRow`/`NestedSheet`/`Song` and `queries::fetch_sheets_for_song` from the candidate-1 typed-queries plan — same names, same signatures.
- **No placeholders** — the one draft placeholder block that appeared mid-task (Step 3's `already_included_song_pks`) is explicitly removed by the very next diff in the same step, not left in the final plan; every other block is complete, real code.

---

**Plan complete and saved to `docs/superpowers/plans/2026-09-07-server-sync-tier.md`.** Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
