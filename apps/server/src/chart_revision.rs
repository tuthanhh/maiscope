//! "Apply a chart revision" (ticket 09): upsert `charts`, append to
//! `chart_revisions`, bump the revision the sync tier reads. One function so
//! `bin/seed_songs.rs` and phase 2's `POST /contributions/{id}/approve` (the
//! part of the write path that actually carries forward — see the ticket)
//! share it instead of each hand-rolling their own SQL.
//!
//! Note the asymmetry with `catalog_sync`, the other writer on
//! `catalog_meta.revision`: this module bumps it with `UPDATE ... RETURNING`,
//! taking the singleton's row lock at the moment it writes. `catalog_sync`
//! takes the same lock at the *start* of its transaction, because it has to
//! stamp rows with the next value long before it knows whether the run
//! changed anything. Either way the lock is what keeps the two from
//! overwriting each other.

use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use std::collections::HashMap;

/// Whether a call actually wrote. Callers that report counts (`seed_songs`)
/// need to tell a real revision from a re-seed of identical bytes — without
/// this they can only count attempts, which makes a no-op run look like it
/// rewrote everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartRevisionOutcome {
    Applied,
    Unchanged,
}

/// Upserts the canonical chart for `(sheet_id, format)`, appends a
/// `chart_revisions` row, and — unless `content` is unchanged from what's
/// already stored — bumps `catalog_meta.revision` and stamps the sheet
/// with it.
///
/// Revision numbers are a shared sequence, not an independent per-row
/// counter: `bin/sync_catalog` draws a fresh `catalog_meta.revision` on every
/// run that changed something and stamps every changed row with that same
/// number. A client doing `GET /sync/delta?since=N` compares against that
/// shared sequence (`sheets.revision > $1`), so `sheets.revision` has to advance *within*
/// the same numbering space `catalog_meta.revision` uses — an independent
/// `sheets.revision = sheets.revision + 1` would put it in a different
/// space and could stay silently below whatever `since` a client already
/// holds, hiding the change from delta polls entirely.
///
/// Returns [`ChartRevisionOutcome::Unchanged`] without writing (no rows, no
/// revision bump) when `content` hashes to what's already stored —
/// re-seeding unchanged bytes shouldn't churn the revision counter or the
/// client-visible ETag (`GET /catalog`, contract §1) on every run.
///
/// The content hash is computed here rather than taken as a parameter:
/// it is derived entirely from `content`, and letting callers supply it
/// makes a mismatched pair — the wrong hash stored against the right
/// bytes — expressible at all, which silently breaks the no-op check above.
pub async fn apply_chart_revision(
    tx: &mut Transaction<'_, Postgres>,
    sheet_id: i64,
    sheet_expr: &str,
    format: &str,
    content: &str,
) -> Result<ChartRevisionOutcome, sqlx::Error> {
    let hash = content_hash(content);

    let unchanged: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM charts WHERE sheet_id = $1 AND format = $2 AND hash = $3)",
    )
    .bind(sheet_id)
    .bind(format)
    .bind(&hash)
    .fetch_one(&mut **tx)
    .await?;

    if unchanged {
        return Ok(ChartRevisionOutcome::Unchanged);
    }

    let chart_id: i64 = sqlx::query_scalar(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash, approved_at) \
         VALUES ($1, $2, $3, $4, $5, now()) \
         ON CONFLICT (sheet_id, format) DO UPDATE \
           SET content = EXCLUDED.content, \
               hash = EXCLUDED.hash, \
               version = charts.version + 1, \
               updated_at = now() \
         RETURNING id",
    )
    .bind(sheet_id)
    .bind(sheet_expr)
    .bind(format)
    .bind(content)
    .bind(&hash)
    .fetch_one(&mut **tx)
    .await?;

    sqlx::query("INSERT INTO chart_revisions (chart_id, content, hash) VALUES ($1, $2, $3)")
        .bind(chart_id)
        .bind(content)
        .bind(&hash)
        .execute(&mut **tx)
        .await?;

    let new_revision: i64 =
        sqlx::query_scalar("UPDATE catalog_meta SET revision = revision + 1 RETURNING revision")
            .fetch_one(&mut **tx)
            .await?;

    sqlx::query("UPDATE sheets SET revision = $1 WHERE id = $2")
        .bind(new_revision)
        .bind(sheet_id)
        .execute(&mut **tx)
        .await?;

    Ok(ChartRevisionOutcome::Applied)
}

/// One chart for [`apply_chart_revisions`]. Owns its strings: the bulk caller
/// builds the whole batch before opening a transaction, so the `maidata.txt`
/// buffers each `content` came from are long gone by then.
#[derive(Debug, Clone)]
pub struct ChartToApply {
    pub sheet_id: i64,
    pub sheet_expr: String,
    pub content: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BulkChartOutcome {
    /// Charts actually written.
    pub applied: usize,
    /// Charts whose stored content already hashed to the same value.
    pub unchanged: usize,
}

/// Bulk sibling of [`apply_chart_revision`]: same semantics for a whole batch,
/// in a fixed number of statements instead of a fixed number *per chart*.
///
/// [`apply_chart_revision`] is 5 round trips per chart plus its own transaction,
/// which is right for the one-at-a-time write path it exists for. Seeding is the
/// other shape: `bin/seed_songs` applies ~6.5k charts in one run, and per-chart
/// round trips put a full seed at ~56k of them — 15-30 minutes against a remote
/// Postgres, where this is seconds. Same reasoning as `catalog_sync`; see that
/// module's header.
///
/// **`catalog_meta.revision` is bumped once for the whole batch, not once per
/// chart.** Every chart written is stamped with that single value. This is a
/// deliberate difference from calling [`apply_chart_revision`] in a loop, which
/// advanced the counter 6.5k times in one seed. One bump per run that changed
/// something is what `catalog_sync` already does, and delta sync is unaffected:
/// a client asking for changes `since=N` still sees every stamped sheet, because
/// they all sit above `N` together.
///
/// Returns zero `applied` and writes nothing at all — no `chart_revisions` row,
/// no revision bump — when every chart in the batch is unchanged.
pub async fn apply_chart_revisions(
    tx: &mut Transaction<'_, Postgres>,
    format: &str,
    charts: &[ChartToApply],
) -> Result<BulkChartOutcome, sqlx::Error> {
    let mut outcome = BulkChartOutcome::default();
    if charts.is_empty() {
        return Ok(outcome);
    }

    // `charts` has UNIQUE (sheet_id, format), so one sheet appearing twice in a
    // single statement would abort it with "ON CONFLICT DO UPDATE command cannot
    // affect row a second time". That is reachable: 78 catalog titles are held by
    // more than one song, so two song directories can resolve to the same song and
    // hence the same sheet. Keep the last occurrence, matching what a loop over
    // `apply_chart_revision` would have left behind.
    let mut deduped: Vec<&ChartToApply> = Vec::with_capacity(charts.len());
    let mut at: HashMap<i64, usize> = HashMap::with_capacity(charts.len());
    for chart in charts {
        match at.get(&chart.sheet_id) {
            Some(&i) => deduped[i] = chart,
            None => {
                at.insert(chart.sheet_id, deduped.len());
                deduped.push(chart);
            }
        }
    }

    // Every stored hash for these sheets in one round trip, replacing the
    // per-chart `SELECT EXISTS(...)`.
    let sheet_ids: Vec<i64> = deduped.iter().map(|c| c.sheet_id).collect();
    let stored: HashMap<i64, String> = sqlx::query_as(
        "SELECT sheet_id, hash FROM charts WHERE format = $1 AND sheet_id = ANY($2)",
    )
    .bind(format)
    .bind(&sheet_ids)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .collect();

    let mut ids: Vec<i64> = Vec::new();
    let mut exprs: Vec<String> = Vec::new();
    let mut contents: Vec<String> = Vec::new();
    let mut hashes: Vec<String> = Vec::new();

    for chart in &deduped {
        let hash = content_hash(&chart.content);
        if stored.get(&chart.sheet_id) == Some(&hash) {
            outcome.unchanged += 1;
            continue;
        }
        ids.push(chart.sheet_id);
        exprs.push(chart.sheet_expr.clone());
        contents.push(chart.content.clone());
        hashes.push(hash);
    }

    outcome.applied = ids.len();
    if ids.is_empty() {
        return Ok(outcome);
    }

    // `version` increments per chart row, so the batch cannot collapse it into a
    // single value — `charts.version + 1` is evaluated per conflicting row.
    let written: Vec<(i64, i64)> = sqlx::query_as(
        "INSERT INTO charts (sheet_id, sheet_expr, format, content, hash, approved_at) \
         SELECT t.sheet_id, t.sheet_expr, $5::text, t.content, t.hash, now() \
         FROM unnest($1::bigint[], $2::text[], $3::text[], $4::text[]) \
              AS t(sheet_id, sheet_expr, content, hash) \
         ON CONFLICT (sheet_id, format) DO UPDATE \
           SET content = EXCLUDED.content, \
               hash = EXCLUDED.hash, \
               version = charts.version + 1, \
               updated_at = now() \
         RETURNING id, sheet_id",
    )
    .bind(&ids)
    .bind(&exprs)
    .bind(&contents)
    .bind(&hashes)
    .bind(format)
    .fetch_all(&mut **tx)
    .await?;

    // chart_revisions is append-only history, keyed by the chart id the upsert
    // just returned. RETURNING order is not specified, so pair by sheet_id rather
    // than by position.
    let chart_id_by_sheet: HashMap<i64, i64> =
        written.into_iter().map(|(id, sheet)| (sheet, id)).collect();
    let mut revision_chart_ids: Vec<i64> = Vec::with_capacity(ids.len());
    for sheet_id in &ids {
        revision_chart_ids.push(chart_id_by_sheet[sheet_id]);
    }

    sqlx::query(
        "INSERT INTO chart_revisions (chart_id, content, hash) \
         SELECT * FROM unnest($1::bigint[], $2::text[], $3::text[])",
    )
    .bind(&revision_chart_ids)
    .bind(&contents)
    .bind(&hashes)
    .execute(&mut **tx)
    .await?;

    let new_revision: i64 =
        sqlx::query_scalar("UPDATE catalog_meta SET revision = revision + 1 RETURNING revision")
            .fetch_one(&mut **tx)
            .await?;

    sqlx::query("UPDATE sheets SET revision = $1 WHERE id = ANY($2)")
        .bind(new_revision)
        .bind(&ids)
        .execute(&mut **tx)
        .await?;

    Ok(outcome)
}

fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn seed_song_with_sheet(pool: &PgPool) -> (i64, String) {
        let song_id: i64 = sqlx::query_scalar(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("maimai_song")
        .bind("Example Song")
        .bind(0)
        .fetch_one(pool)
        .await
        .unwrap();

        let sheet_expr = "maimai_song|dx|master".to_string();
        let sheet_id: i64 = sqlx::query_scalar(
            "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index) \
             VALUES ($1, $2, 'dx', 'master', 0) RETURNING id",
        )
        .bind(song_id)
        .bind(&sheet_expr)
        .fetch_one(pool)
        .await
        .unwrap();

        (sheet_id, sheet_expr)
    }

    async fn catalog_meta_revision(pool: &PgPool) -> i64 {
        sqlx::query_scalar("SELECT revision FROM catalog_meta LIMIT 1")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // The caller must be able to tell a real write from a no-op: seed_songs
    // reports "seeded N" for CI, and counting no-ops as writes makes a
    // re-run of unchanged data look like it did N times more than it did.
    #[sqlx::test]
    async fn outcome_distinguishes_a_write_from_a_no_op(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        let first =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        assert_eq!(first, ChartRevisionOutcome::Applied);

        let mut tx = pool.begin().await?;
        let repeat =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        assert_eq!(repeat, ChartRevisionOutcome::Unchanged);

        let mut tx = pool.begin().await?;
        let changed =
            apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v2").await?;
        tx.commit().await?;
        assert_eq!(changed, ChartRevisionOutcome::Applied);
        Ok(())
    }

    // The hash is the function's own business — callers pass content only.
    // Compared against sha256("v1") computed outside this codebase, so the
    // assertion cannot agree with a wrong implementation.
    #[sqlx::test]
    async fn content_is_hashed_with_sha256_by_the_function(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let stored: String = sqlx::query_scalar("SELECT hash FROM charts WHERE sheet_id = $1")
            .bind(sheet_id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            stored,
            "3bfc269594ef649228e9a74bab00f042efc91d5acc6fbee31a382e80d42388fe"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_a_revision_twice_appends_two_chart_revisions_rows(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v2").await?;
        tx.commit().await?;

        let chart_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM charts")
            .fetch_one(&pool)
            .await?;
        let revision_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            chart_count, 1,
            "UNIQUE(sheet_id, format) — second call updates, not inserts"
        );
        assert_eq!(
            revision_row_count, 2,
            "append-only — both calls add a history row"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_a_revision_bumps_catalog_meta_and_sheet_revision(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;
        let before = catalog_meta_revision(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let after = catalog_meta_revision(&pool).await;
        assert_eq!(after, before + 1);

        let sheet_revision: i64 = sqlx::query_scalar("SELECT revision FROM sheets WHERE id = $1")
            .bind(sheet_id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            sheet_revision, after,
            "sheet must share catalog_meta's numbering, not its own independent counter"
        );
        Ok(())
    }

    // ── bulk path ────────────────────────────────────────────────────────────

    async fn meta(pool: &PgPool) {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(pool)
            .await
            .unwrap();
    }

    /// `n` sheets under one song, so a batch has more than one row to get wrong.
    async fn seed_sheets(pool: &PgPool, n: usize) -> Vec<(i64, String)> {
        let song_id: i64 = sqlx::query_scalar(
            "INSERT INTO songs (song_id, title, source_index) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("maimai_song")
        .bind("Example Song")
        .bind(0)
        .fetch_one(pool)
        .await
        .unwrap();

        let mut out = Vec::new();
        for i in 0..n {
            let expr = format!("maimai_song|dx|d{i}");
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO sheets (song_id_fk, sheet_expr, type, difficulty, source_index) \
                 VALUES ($1, $2, 'dx', $3, $4) RETURNING id",
            )
            .bind(song_id)
            .bind(&expr)
            .bind(format!("d{i}"))
            .bind(i as i32)
            .fetch_one(pool)
            .await
            .unwrap();
            out.push((id, expr));
        }
        out
    }

    fn to_apply(sheets: &[(i64, String)], content: &str) -> Vec<ChartToApply> {
        sheets
            .iter()
            .map(|(id, expr)| ChartToApply {
                sheet_id: *id,
                sheet_expr: expr.clone(),
                content: content.to_string(),
            })
            .collect()
    }

    // The reason this function exists: a loop over apply_chart_revision advanced
    // catalog_meta.revision once per chart, so one seed of ~6.5k charts moved the
    // counter 6.5k times. One bump per run that changed something is what
    // catalog_sync already does.
    #[sqlx::test]
    async fn a_bulk_apply_bumps_the_revision_once_for_the_whole_batch(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 5).await;
        let before = catalog_meta_revision(&pool).await;

        let mut tx = pool.begin().await?;
        let outcome =
            apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;

        assert_eq!(outcome.applied, 5);
        assert_eq!(outcome.unchanged, 0);

        let after = catalog_meta_revision(&pool).await;
        assert_eq!(after, before + 1, "one bump, not one per chart");

        // Every written sheet carries that single revision, so a client polling
        // since=before sees all five together.
        let revisions: Vec<i64> =
            sqlx::query_scalar("SELECT DISTINCT revision FROM sheets ORDER BY revision")
                .fetch_all(&pool)
                .await?;
        assert_eq!(revisions, vec![after]);
        Ok(())
    }

    #[sqlx::test]
    async fn a_bulk_apply_writes_every_chart_and_its_history(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 4).await;

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;

        // Pair content to the right sheet — a misaligned unnest would still write
        // four rows, just with the contents shuffled between them.
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT s.sheet_expr, c.content FROM charts c \
             JOIN sheets s ON s.id = c.sheet_id ORDER BY s.sheet_expr",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(rows.len(), 4);
        for (expr, content) in &rows {
            assert_eq!(content, "v1", "{expr} got the wrong content");
        }

        let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(history, 4);
        Ok(())
    }

    // Distinct content per sheet: this is what catches a chart_revisions row
    // paired against the wrong chart_id, which a uniform-content batch cannot see.
    #[sqlx::test]
    async fn bulk_history_rows_pair_with_the_right_chart(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 3).await;
        let charts: Vec<ChartToApply> = sheets
            .iter()
            .enumerate()
            .map(|(i, (id, expr))| ChartToApply {
                sheet_id: *id,
                sheet_expr: expr.clone(),
                content: format!("content-{i}"),
            })
            .collect();

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &charts).await?;
        tx.commit().await?;

        let paired: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT s.sheet_expr, c.content, r.content FROM chart_revisions r \
             JOIN charts c ON c.id = r.chart_id \
             JOIN sheets s ON s.id = c.sheet_id ORDER BY s.sheet_expr",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(paired.len(), 3);
        for (expr, chart_content, history_content) in paired {
            assert_eq!(
                chart_content, history_content,
                "{expr}: history row is attached to the wrong chart"
            );
        }
        Ok(())
    }

    #[sqlx::test]
    async fn a_bulk_rerun_of_identical_content_writes_nothing(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 3).await;

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;
        let revision_after_first = catalog_meta_revision(&pool).await;

        let mut tx = pool.begin().await?;
        let outcome =
            apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;

        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.unchanged, 3);
        assert_eq!(catalog_meta_revision(&pool).await, revision_after_first);
        let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(history, 3, "no second history row for unchanged bytes");
        Ok(())
    }

    #[sqlx::test]
    async fn a_bulk_apply_reports_applied_and_unchanged_separately(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 3).await;

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;

        // Change exactly one of the three.
        let mut mixed = to_apply(&sheets, "v1");
        mixed[1].content = "v2".to_string();

        let mut tx = pool.begin().await?;
        let outcome = apply_chart_revisions(&mut tx, "maimai-simai", &mixed).await?;
        tx.commit().await?;

        assert_eq!(outcome.applied, 1);
        assert_eq!(outcome.unchanged, 2);

        // Only the changed sheet advances — that is what /sync/delta reports on.
        let bumped: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sheets WHERE revision = (SELECT revision FROM catalog_meta LIMIT 1)",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(bumped, 1);
        Ok(())
    }

    // charts has UNIQUE (sheet_id, format), so one sheet twice in a single
    // statement aborts it with "ON CONFLICT DO UPDATE command cannot affect row a
    // second time". Reachable in seeding: 78 catalog titles are held by more than
    // one song, so two song directories can resolve to the same sheet.
    #[sqlx::test]
    async fn a_repeated_sheet_in_one_batch_collapses_rather_than_failing(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 1).await;
        let (id, expr) = sheets[0].clone();

        let batch = vec![
            ChartToApply {
                sheet_id: id,
                sheet_expr: expr.clone(),
                content: "first".into(),
            },
            ChartToApply {
                sheet_id: id,
                sheet_expr: expr.clone(),
                content: "second".into(),
            },
        ];

        let mut tx = pool.begin().await?;
        let outcome = apply_chart_revisions(&mut tx, "maimai-simai", &batch).await?;
        tx.commit().await?;

        assert_eq!(outcome.applied, 1, "collapsed to one write");
        let content: String = sqlx::query_scalar("SELECT content FROM charts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            content, "second",
            "last occurrence wins, as a loop would leave"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn an_empty_batch_writes_nothing(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let before = catalog_meta_revision(&pool).await;

        let mut tx = pool.begin().await?;
        let outcome = apply_chart_revisions(&mut tx, "maimai-simai", &[]).await?;
        tx.commit().await?;

        assert_eq!(outcome, BulkChartOutcome::default());
        assert_eq!(catalog_meta_revision(&pool).await, before);
        Ok(())
    }

    // The bulk path must agree with the single path byte for byte, or a chart
    // seeded in bulk would look changed the next time the write endpoint touched it.
    #[sqlx::test]
    async fn bulk_and_single_agree_on_the_stored_hash(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 1).await;

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;

        let stored: String = sqlx::query_scalar("SELECT hash FROM charts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(
            stored, "3bfc269594ef649228e9a74bab00f042efc91d5acc6fbee31a382e80d42388fe",
            "same sha256 the single path asserts"
        );

        // And the single path now sees it as unchanged.
        let mut tx = pool.begin().await?;
        let outcome =
            apply_chart_revision(&mut tx, sheets[0].0, &sheets[0].1, "maimai-simai", "v1").await?;
        tx.commit().await?;
        assert_eq!(outcome, ChartRevisionOutcome::Unchanged);
        Ok(())
    }

    // charts.version increments per row; a batch must not collapse it to one value.
    #[sqlx::test]
    async fn bulk_version_increments_per_chart_not_per_batch(pool: PgPool) -> sqlx::Result<()> {
        meta(&pool).await;
        let sheets = seed_sheets(&pool, 3).await;

        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v1")).await?;
        tx.commit().await?;
        let mut tx = pool.begin().await?;
        apply_chart_revisions(&mut tx, "maimai-simai", &to_apply(&sheets, "v2")).await?;
        tx.commit().await?;

        let versions: Vec<i32> = sqlx::query_scalar("SELECT version FROM charts ORDER BY sheet_id")
            .fetch_all(&pool)
            .await?;
        assert_eq!(
            versions,
            vec![2, 2, 2],
            "each row advanced once per rewrite"
        );
        Ok(())
    }

    #[sqlx::test]
    async fn applying_unchanged_content_is_a_no_op(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO catalog_meta (id, update_time) VALUES (true, now())")
            .execute(&pool)
            .await?;
        let (sheet_id, sheet_expr) = seed_song_with_sheet(&pool).await;

        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;
        let revision_after_first = catalog_meta_revision(&pool).await;

        // Same hash — re-seeding identical bytes shouldn't churn anything.
        let mut tx = pool.begin().await?;
        apply_chart_revision(&mut tx, sheet_id, &sheet_expr, "maimai-simai", "v1").await?;
        tx.commit().await?;

        let revision_after_second = catalog_meta_revision(&pool).await;
        let revision_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chart_revisions")
            .fetch_one(&pool)
            .await?;
        assert_eq!(revision_after_second, revision_after_first);
        assert_eq!(revision_row_count, 1);
        Ok(())
    }
}
