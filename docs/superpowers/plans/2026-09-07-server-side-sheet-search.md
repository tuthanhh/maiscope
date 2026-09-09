# Server-Side Sheet Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `GET /api/v1/sheets/search` endpoint to `apps/server` that runs the browse page's filter/search logic in SQL — with pagination — instead of the frontend fetching the whole catalog and filtering client-side in JS.

**Architecture:** One new handler in `apps/server/src/main.rs` backed by one new typed query function in `apps/server/src/queries.rs`, reusing the `types::Sheet`/`types::SongRow`/`types::SheetRow` seam this repo's candidate-1 typed-query migration already established (see `docs/superpowers/plans/2026-09-07-server-catalog-typed-queries.md` — this plan assumes that one has landed first, since it depends on `types::RegionOverride`, `SheetRow::into_meta`, and the `has_chart` column already being wired in). The query builds one dynamic `WHERE` clause (via `sqlx::QueryBuilder`, since the filter set is optional/combinatorial — `query_as!`'s compile-time-checked macro can't express "N optional predicates," so this one query is built at runtime and loses compile-time column checking; every other query in this codebase keeps `query_as!`). Region-override remapping is a `LEFT JOIN sheet_region_overrides` with per-field `COALESCE`.

**Tech Stack:** Rust, Axum, sqlx 0.9 (`QueryBuilder` for the dynamic filter query), Postgres.

**Spec:** No separate spec document — encodes the design reached via `/grilling` in-session (the server-side-filtering discussion following the 2026-09-07 architecture review). Key decisions:
- New dedicated endpoint, not an extension of `GET /catalog` (which stays the full-snapshot endpoint for the local-first cache).
- `superFilter` (arbitrary eval'd JS) is being removed from the app entirely (see the companion frontend plan) — not supported here, full stop.
- All other `Filters` fields get support here, including `region`/`useRegionOverride`/`matchExactTitle`/`matchExactArtist`/`useInternalLevel` even though today's UI doesn't expose them yet — a UI for them is being added in the companion frontend plan.
- Category matching must handle the `|`-delimited multi-category column (`sheet.category` can be `"action|anime"`).
- Response includes `total` for pagination; default sort matches `GET /catalog`'s order (song `source_index`, then sheet `source_index`) since no sort UI exists.

## Global Constraints

- Never touch production — local dev Postgres only (`docker compose up -d` in `apps/server`).
- Depends on the candidate-1 typed-query migration (`docs/superpowers/plans/2026-09-07-server-catalog-typed-queries.md`) having landed — this plan uses `types::SongRow`, `types::SheetRow`, `types::RegionOverride`, and `SheetRow::into_meta()`/`SongRow::into_meta()` from that plan's Task 3.
- Doc sync rule (CLAUDE.md): `apps/server/docs/api-contract.md` must document this endpoint in the same change (Task 3 below).
- Server returns raw fields only — no client-derived fields in the response.

---

### Task 1: `queries.rs` — the dynamic search query

**Files:**
- Modify: `apps/server/src/queries.rs`

**Interfaces:**
- Consumes: `types::{SongRow, SheetRow, RegionOverride}` (from the candidate-1 plan's Task 3).
- Produces:
  - `pub struct SheetSearchParams { pub title: Option<String>, pub match_exact_title: bool, pub artist: Option<String>, pub match_exact_artist: bool, pub categories: Vec<String>, pub versions: Vec<String>, pub types: Vec<String>, pub difficulties: Vec<String>, pub min_level_value: Option<f64>, pub max_level_value: Option<f64>, pub use_internal_level: bool, pub min_bpm: Option<f64>, pub max_bpm: Option<f64>, pub note_designers: Vec<String>, pub region: Option<String>, pub use_region_override: bool, pub page: i64, pub page_size: i64 }`
  - `pub async fn search_sheets(pool: &PgPool, params: &SheetSearchParams) -> Result<(Vec<(SongRow, SheetRow)>, i64), sqlx::Error>` — returns `(rows, total_matching_before_pagination)`.

- [ ] **Step 1: Add the params struct and a failing test**

```rust
pub struct SheetSearchParams {
    pub title: Option<String>,
    pub match_exact_title: bool,
    pub artist: Option<String>,
    pub match_exact_artist: bool,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub types: Vec<String>,
    pub difficulties: Vec<String>,
    pub min_level_value: Option<f64>,
    pub max_level_value: Option<f64>,
    pub use_internal_level: bool,
    pub min_bpm: Option<f64>,
    pub max_bpm: Option<f64>,
    pub note_designers: Vec<String>,
    pub region: Option<String>,
    pub use_region_override: bool,
    pub page: i64,
    pub page_size: i64,
}

impl Default for SheetSearchParams {
    fn default() -> Self {
        Self {
            title: None,
            match_exact_title: false,
            artist: None,
            match_exact_artist: false,
            categories: Vec::new(),
            versions: Vec::new(),
            types: Vec::new(),
            difficulties: Vec::new(),
            min_level_value: None,
            max_level_value: None,
            use_internal_level: false,
            min_bpm: None,
            max_bpm: None,
            note_designers: Vec::new(),
            region: None,
            use_region_override: false,
            page: 1,
            page_size: 22,
        }
    }
}

pub async fn search_sheets(
    pool: &PgPool,
    params: &SheetSearchParams,
) -> Result<(Vec<(crate::types::SongRow, crate::types::SheetRow)>, i64), sqlx::Error> {
    todo!()
}
```

Test (append to `#[cfg(test)] mod tests` in `queries.rs`):

```rust
    async fn seed_two_songs_for_search(pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, artist, category, version, bpm, source_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            "song_a", "Fire Flower", "Composer A", "pops", "maimai DX", 180.0_f64, 0
        )
        .execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO songs (song_id, title, artist, category, version, bpm, source_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            "song_b", "Ice Crystal", "Composer B", "anime", "maimai DX", 140.0_f64, 1
        )
        .execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level_value, source_index)
             SELECT id, $1, 'dx', 'master', 13.5, 0 FROM songs WHERE song_id = $2",
            "song_a|dx|master", "song_a"
        )
        .execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, level_value, source_index)
             SELECT id, $1, 'dx', 'master', 11.0, 0 FROM songs WHERE song_id = $2",
            "song_b|dx|master", "song_b"
        )
        .execute(pool).await.unwrap();
    }

    #[sqlx::test]
    async fn search_sheets_filters_by_title_substring(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams { title: Some("fire".to_string()), ..Default::default() };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Fire Flower"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_filters_by_level_range(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams { min_level_value: Some(12.0), ..Default::default() };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Fire Flower"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_paginates_and_reports_total(pool: PgPool) -> sqlx::Result<()> {
        seed_two_songs_for_search(&pool).await;

        let params = SheetSearchParams { page: 1, page_size: 1, ..Default::default() };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 2); // total matches regardless of page_size
        assert_eq!(rows.len(), 1);
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_matches_pipe_delimited_category(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, category, source_index) VALUES ($1, $2, $3, $4)",
            "song_c", "Dual Genre", "pops|anime", 0
        )
        .execute(&pool).await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, source_index)
             SELECT id, $1, 0 FROM songs WHERE song_id = $2",
            "song_c|dx|master", "song_c"
        )
        .execute(&pool).await?;

        let params = SheetSearchParams { categories: vec!["anime".to_string()], ..Default::default() };
        let (rows, total) = search_sheets(&pool, &params).await.unwrap();

        assert_eq!(total, 1);
        assert_eq!(rows[0].0.title.as_deref(), Some("Dual Genre"));
        Ok(())
    }

    #[sqlx::test]
    async fn search_sheets_applies_region_override_before_level_filter(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "song_d", "Region Song", 0
        )
        .execute(&pool).await?;
        let sheet_pk: i64 = sqlx::query_scalar!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, level_value, source_index)
             SELECT id, $1, 10.0, 0 FROM songs WHERE song_id = $2 RETURNING id",
            "song_d|dx|master", "song_d"
        )
        .fetch_one(&pool).await?;
        // In the "jp" region this sheet is actually level 14.0.
        sqlx::query!(
            "INSERT INTO sheet_region_overrides (sheet_id, region, level_value) VALUES ($1, 'jp', 14.0)",
            sheet_pk
        )
        .execute(&pool).await?;

        // Without the override: min_level_value 12 excludes it (base is 10.0).
        let base_params = SheetSearchParams { min_level_value: Some(12.0), ..Default::default() };
        let (_, total_base) = search_sheets(&pool, &base_params).await.unwrap();
        assert_eq!(total_base, 0);

        // With region=jp + useRegionOverride: the override's 14.0 satisfies min 12.
        let override_params = SheetSearchParams {
            min_level_value: Some(12.0),
            region: Some("jp".to_string()),
            use_region_override: true,
            ..Default::default()
        };
        let (_, total_override) = search_sheets(&pool, &override_params).await.unwrap();
        assert_eq!(total_override, 1);
        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/server && cargo test --lib search_sheets`
Expected: FAIL (`not yet implemented`).

- [ ] **Step 3: Implement `search_sheets` with `sqlx::QueryBuilder`**

```rust
pub async fn search_sheets(
    pool: &PgPool,
    params: &SheetSearchParams,
) -> Result<(Vec<(crate::types::SongRow, crate::types::SheetRow)>, i64), sqlx::Error> {
    use sqlx::QueryBuilder;

    // Region-override-aware effective columns: when a positive region is
    // selected and use_region_override is set, the override's non-null
    // columns win; otherwise fall back to the sheet's own value.
    let effective_region = params
        .region
        .as_deref()
        .filter(|r| params.use_region_override && !r.starts_with('!'));

    fn build_where(qb: &mut QueryBuilder<'_, sqlx::Postgres>, params: &SheetSearchParams, effective_region: Option<&str>) {
        qb.push(" WHERE 1=1 ");

        if let Some(title) = &params.title {
            if params.match_exact_title {
                qb.push(" AND so.title = ").push_bind(title.clone());
            } else {
                qb.push(" AND so.title ILIKE ").push_bind(format!("%{title}%"));
            }
        }
        if let Some(artist) = &params.artist {
            if params.match_exact_artist {
                qb.push(" AND so.artist = ").push_bind(artist.clone());
            } else {
                qb.push(" AND so.artist ILIKE ").push_bind(format!("%{artist}%"));
            }
        }
        if !params.categories.is_empty() {
            qb.push(" AND string_to_array(so.category, '|') && ")
                .push_bind(params.categories.clone());
        }
        if !params.versions.is_empty() {
            qb.push(" AND so.version = ANY(").push_bind(params.versions.clone()).push(")");
        }
        if !params.types.is_empty() {
            qb.push(" AND s.type = ANY(").push_bind(params.types.clone()).push(")");
        }
        if !params.difficulties.is_empty() {
            qb.push(" AND s.difficulty = ANY(").push_bind(params.difficulties.clone()).push(")");
        }
        if !params.note_designers.is_empty() {
            let designer_col = if effective_region.is_some() {
                "COALESCE(sro.note_designer, s.note_designer)"
            } else {
                "s.note_designer"
            };
            qb.push(" AND ").push(designer_col).push(" = ANY(").push_bind(params.note_designers.clone()).push(")");
        }
        let level_col = match (params.use_internal_level, effective_region.is_some()) {
            (true, true) => "COALESCE(sro.internal_level_value, s.internal_level_value)",
            (true, false) => "s.internal_level_value",
            (false, true) => "COALESCE(sro.level_value, s.level_value)",
            (false, false) => "s.level_value",
        };
        if let Some(min_level) = params.min_level_value {
            qb.push(" AND ").push(level_col).push(" >= ").push_bind(min_level);
        }
        if let Some(max_level) = params.max_level_value {
            qb.push(" AND ").push(level_col).push(" <= ").push_bind(max_level);
        }
        if let Some(min_bpm) = params.min_bpm {
            qb.push(" AND so.bpm >= ").push_bind(min_bpm);
        }
        if let Some(max_bpm) = params.max_bpm {
            qb.push(" AND so.bpm <= ").push_bind(max_bpm);
        }
        if let Some(region) = &params.region {
            if let Some(excluded) = region.strip_prefix('!') {
                qb.push(" AND (sr_excl.available IS NULL OR sr_excl.available = false) ");
                // sr_excl is joined conditionally below via build_joins; this
                // branch only adds the predicate, not the join.
                let _ = excluded; // the join binds the region value, see build_joins
            } else {
                qb.push(" AND sr_incl.available = true ");
            }
        }
    }

    fn build_joins(qb: &mut QueryBuilder<'_, sqlx::Postgres>, params: &SheetSearchParams, effective_region: Option<&str>) {
        if let Some(region) = effective_region {
            qb.push(" LEFT JOIN sheet_region_overrides sro ON sro.sheet_id = s.id AND sro.region = ")
                .push_bind(region.to_string());
        }
        if let Some(region) = &params.region {
            if let Some(excluded) = region.strip_prefix('!') {
                qb.push(" LEFT JOIN sheet_regions sr_excl ON sr_excl.sheet_id = s.id AND sr_excl.region = ")
                    .push_bind(excluded.to_string());
            } else {
                qb.push(" JOIN sheet_regions sr_incl ON sr_incl.sheet_id = s.id AND sr_incl.region = ")
                    .push_bind(region.clone());
            }
        }
    }

    // ── total count ──
    let mut count_qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM sheets s JOIN songs so ON so.id = s.song_id_fk ");
    build_joins(&mut count_qb, params, effective_region);
    build_where(&mut count_qb, params, effective_region);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    // ── page of rows ──
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        r#"SELECT
            so.id, so.song_id, so.category, so.title, so.artist, so.bpm, so.image_name, so.version,
            to_char(so.release_date, 'YYYY-MM-DD') AS release_date, so.is_new, so.is_locked, so.comment,
            s.song_id_fk, s.type, s.difficulty, s.level, s.level_value, s.internal_level,
            s.internal_level_value, s.note_designer, s.is_special,
            EXISTS (SELECT 1 FROM charts c WHERE c.sheet_id = s.id AND c.content IS NOT NULL) AS has_chart,
            (SELECT json_object_agg(key, value) FROM sheet_note_counts WHERE sheet_id = s.id) AS note_counts,
            (SELECT json_object_agg(region, available) FROM sheet_regions WHERE sheet_id = s.id) AS regions,
            (SELECT json_object_agg(region, json_build_object(
                'level', level, 'levelValue', level_value,
                'internalLevel', internal_level, 'internalLevelValue', internal_level_value,
                'noteDesigner', note_designer
            )) FROM sheet_region_overrides WHERE sheet_id = s.id) AS region_overrides
           FROM sheets s JOIN songs so ON so.id = s.song_id_fk "#,
    );
    build_joins(&mut qb, params, effective_region);
    build_where(&mut qb, params, effective_region);
    qb.push(" ORDER BY so.source_index, s.source_index ");
    qb.push(" LIMIT ").push_bind(params.page_size);
    qb.push(" OFFSET ").push_bind((params.page - 1).max(0) * params.page_size);

    let rows = qb
        .build_query_as::<(
            crate::types::SongRow,
            i64, // song_id_fk (discarded, needed to satisfy column order)
            Option<String>, Option<String>, Option<String>, Option<f64>, Option<String>,
            Option<f64>, Option<String>, Option<bool>, bool,
            Option<sqlx::types::Json<std::collections::BTreeMap<String, Option<i64>>>>,
            Option<sqlx::types::Json<std::collections::BTreeMap<String, bool>>>,
            Option<sqlx::types::Json<std::collections::BTreeMap<String, crate::types::RegionOverride>>>,
        )>()
        .fetch_all(pool)
        .await?;

    let result = rows
        .into_iter()
        .map(|(song, song_id_fk, r#type, difficulty, level, level_value, internal_level,
                internal_level_value, note_designer, is_special, has_chart,
                note_counts, regions, region_overrides)| {
            let sheet = crate::types::SheetRow {
                song_id_fk,
                r#type, difficulty, level, level_value, internal_level, internal_level_value,
                note_designer, is_special, has_chart, note_counts, regions, region_overrides,
            };
            (song, sheet)
        })
        .collect();

    Ok((result, total))
}
```

`QueryBuilder::build_query_as` doesn't support the `#[derive(sqlx::FromRow)]`-free `query_as!` macro's positional-struct trick directly for an ad-hoc tuple this wide — `SongRow` must derive `sqlx::FromRow` for this to compile as the first tuple element. Add that derive in this task (it doesn't conflict with candidate-1's plan, which only used `query_as!`, not `FromRow` directly):

```diff
+#[derive(sqlx::FromRow)]
 pub struct SongRow {
     pub id: i64,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/server && cargo test --lib search_sheets`
Expected: PASS (5 passed).

- [ ] **Step 5: Commit**

```bash
git add apps/server/src/queries.rs
git commit -m "feat(server): add search_sheets typed dynamic-filter query"
```

---

### Task 2: `GET /api/v1/sheets/search` handler

**Files:**
- Modify: `apps/server/src/main.rs`

**Interfaces:**
- Consumes: `queries::{SheetSearchParams, search_sheets}` (Task 1); `types::Sheet` (candidate-1 plan).
- Produces: `SheetSearchResponse { sheets: Vec<Sheet>, total: i64 }` (add to `types.rs`), wired to route `GET /sheets/search` under `/api/v1`.

- [ ] **Step 1: Add the response type**

```rust
// apps/server/src/types.rs
#[derive(Debug, Serialize)]
pub struct SheetSearchResponse {
    pub sheets: Vec<Sheet>,
    pub total: i64,
}
```

- [ ] **Step 2: Add the query-param struct and handler in `main.rs`**

```rust
// Query params for GET /sheets/search — mirrors apps/host/src/types/Filters.ts
// (minus superFilter, which is being removed from the app — see the
// companion frontend plan).
#[derive(Debug, Deserialize)]
struct SheetSearchQuery {
    title: Option<String>,
    #[serde(default)]
    match_exact_title: bool,
    artist: Option<String>,
    #[serde(default)]
    match_exact_artist: bool,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    versions: Vec<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    difficulties: Vec<String>,
    min_level_value: Option<f64>,
    max_level_value: Option<f64>,
    #[serde(default)]
    use_internal_level: bool,
    min_bpm: Option<f64>,
    max_bpm: Option<f64>,
    #[serde(default)]
    note_designers: Vec<String>,
    region: Option<String>,
    #[serde(default)]
    use_region_override: bool,
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    22
}

// GET /sheets/search — filtered, paginated sheet list (contract §1.2).
async fn search_sheets(
    Query(q): Query<SheetSearchQuery>,
    State(pool): State<Pool<Postgres>>,
) -> Result<Json<types::SheetSearchResponse>, (StatusCode, Json<Value>)> {
    let params = queries::SheetSearchParams {
        title: q.title,
        match_exact_title: q.match_exact_title,
        artist: q.artist,
        match_exact_artist: q.match_exact_artist,
        categories: q.categories,
        versions: q.versions,
        types: q.types,
        difficulties: q.difficulties,
        min_level_value: q.min_level_value,
        max_level_value: q.max_level_value,
        use_internal_level: q.use_internal_level,
        min_bpm: q.min_bpm,
        max_bpm: q.max_bpm,
        note_designers: q.note_designers,
        region: q.region,
        use_region_override: q.use_region_override,
        page: q.page.max(1),
        page_size: q.page_size.clamp(1, 100),
    };

    let (rows, total) = queries::search_sheets(&pool, &params).await.map_err(db_error)?;

    let sheets = rows
        .into_iter()
        .map(|(song, sheet)| types::Sheet { song: song.into_meta(), sheet: sheet.into_meta() })
        .collect();

    Ok(Json(types::SheetSearchResponse { sheets, total }))
}
```

- [ ] **Step 3: Wire the route**

```diff
                 .route("/sheets/{sheet}", get(get_sheet))
+                .route("/sheets/search", get(search_sheets))
                 .route("/sheets/{sheet}/chart", get(get_chart)),
```

Route ordering matters here: Axum matches `/sheets/search` against the literal-first, so it must be registered before or independent of `/sheets/{sheet}` — Axum 0.8's router resolves static segments before wildcards regardless of registration order, so this is safe either way, but keeping `/sheets/search` visually grouped with the other `/sheets/*` routes is for readability, not correctness.

- [ ] **Step 4: Handler-level integration test**

```rust
    #[sqlx::test]
    async fn search_sheets_handler_returns_paginated_response(pool: sqlx::PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3)",
            "maimai_song", "Example Song", 0
        )
        .execute(&pool).await?;
        sqlx::query!(
            "INSERT INTO sheets (song_id_fk, sheet_expr, source_index)
             SELECT id, $1, 0 FROM songs WHERE song_id = $2",
            "maimai_song|dx|master", "maimai_song"
        )
        .execute(&pool).await?;

        let result = search_sheets(
            AxumQuery(SheetSearchQuery {
                title: None, match_exact_title: false, artist: None, match_exact_artist: false,
                categories: vec![], versions: vec![], types: vec![], difficulties: vec![],
                min_level_value: None, max_level_value: None, use_internal_level: false,
                min_bpm: None, max_bpm: None, note_designers: vec![], region: None,
                use_region_override: false, page: 1, page_size: 22,
            }),
            AxumState(pool),
        )
        .await
        .unwrap();

        assert_eq!(result.0.total, 1);
        assert_eq!(result.0.sheets.len(), 1);
        Ok(())
    }
```

- [ ] **Step 5: Run all server tests**

Run: `cd apps/server && cargo test --lib`
Expected: all pass, including the new handler test.

- [ ] **Step 6: Manual smoke test**

```bash
cd apps/server && docker compose up -d && sqlx migrate run && cargo run &
sleep 1
curl -s "http://localhost:3000/api/v1/sheets/search?title=fire&page=1&pageSize=10" | jq '.total, (.sheets | length)'
kill %1
```

- [ ] **Step 7: Commit**

```bash
git add apps/server/src/main.rs apps/server/src/types.rs
git commit -m "feat(server): add GET /sheets/search endpoint"
```

---

### Task 3: Document the endpoint in `api-contract.md`

**Files:**
- Modify: `apps/server/docs/api-contract.md`

**Interfaces:** none (doc-only).

- [ ] **Step 1: Add a new §1.2 section** (immediately after the existing `GET /songs/{songId}` / `GET /sheets/{sheetExpr}` entries)

```markdown
### `GET /sheets/search`

Filtered, paginated sheet list — powers the browse page's search/filter UI.
Unlike `GET /catalog` (full snapshot for the offline-first cache), this
endpoint does the filtering in SQL and returns only a page of results.

Query params (all optional except none are required — omitting all returns
the full unfiltered, paginated sheet list):

| param | type | meaning |
|-------|------|---------|
| `title` | string | substring match on song title (case-insensitive), or exact match if `matchExactTitle` is set |
| `matchExactTitle` | boolean | see above |
| `artist` | string | same substring/exact behavior as `title`, on artist |
| `matchExactArtist` | boolean | see above |
| `categories` | string[] | matches if any of `sheet.category`'s `\|`-delimited parts is in this list |
| `versions` | string[] | exact match |
| `types` | string[] | exact match |
| `difficulties` | string[] | exact match |
| `minLevelValue` / `maxLevelValue` | number | inclusive range on level value (or internal level value if `useInternalLevel`) |
| `useInternalLevel` | boolean | see above |
| `minBPM` / `maxBPM` | number | inclusive range |
| `noteDesigners` | string[] | exact match |
| `region` | string | prefix `!` excludes; otherwise includes. When combined with `useRegionOverride`, the region's override values (level/internalLevel/noteDesigner) substitute for the base sheet's before other filters evaluate |
| `useRegionOverride` | boolean | see above |
| `page` | integer | 1-indexed, default 1 |
| `pageSize` | integer | default 22, max 100 |

Response `200`:
```jsonc
{
  "sheets": [ /* Sheet[] — same shape as GET /sheets/{sheetExpr}, see §1.1 */ ],
  "total": 0   // total matches before pagination, for computing page count
}
```

No `superFilter` equivalent — the client-side arbitrary-JS filter was removed
from the app (never had UI wiring); revisit if/when the app needs it again.
```

- [ ] **Step 2: Verify**

Run: `grep -n "sheets/search" apps/server/docs/api-contract.md`
Expected: the new section heading is present.

- [ ] **Step 3: Commit**

```bash
git add apps/server/docs/api-contract.md
git commit -m "docs: document GET /sheets/search in api-contract.md"
```

---

## Self-Review Notes

- **Spec coverage:** every `Filters` field except `superFilter` has a query param (Task 1-2); category `|`-split ✓ (Task 1 test `search_sheets_matches_pipe_delimited_category`); region-override `COALESCE` sequencing ✓ (Task 1 test `search_sheets_applies_region_override_before_level_filter`); pagination + `total` ✓ (Task 1 test `search_sheets_paginates_and_reports_total`); doc sync ✓ (Task 3).
- **Dependency note:** this plan assumes `docs/superpowers/plans/2026-09-07-server-catalog-typed-queries.md` has already landed (needs `SongRow`/`SheetRow`/`RegionOverride`/`into_meta()` to exist) — execute that plan first if it hasn't.
- **Type consistency:** `SheetSearchParams` field names match `SheetSearchQuery`'s exactly (just Rust snake_case vs the same names); `search_sheets` (query function) and `search_sheets` (handler) are two different items in two different modules (`queries::search_sheets` vs the free fn in `main.rs`) — Task 2 calls `queries::search_sheets` explicitly to disambiguate.
- **No placeholders** — every SQL string and Rust type above is complete, no "TODO"/"similar to."

---

**Plan complete and saved to `docs/superpowers/plans/2026-09-07-server-side-sheet-search.md`.** Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
